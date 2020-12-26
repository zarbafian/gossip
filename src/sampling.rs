use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::sync::atomic::AtomicBool;
use std::net::SocketAddr;
use rand::Rng;
use rand::seq::SliceRandom;
use std::error::Error;
use std::sync::mpsc::Receiver;
use std::collections::{HashSet, VecDeque};
use std::iter::FromIterator;
use crate::PeerSamplingConfig;
use crate::peer::Peer;
use crate::message::sampling::PeerSamplingMessage;
use crate::message::{NoopMessage, MessageType};

/// Peer sampling service to by used by application
pub struct PeerSamplingService {
    /// Peer address
    address: SocketAddr,
    /// Protocol parameters
    config: PeerSamplingConfig,
    /// View containing a list of other peers
    view: Arc<Mutex<View>>,
    // Handles for activity threads
    thread_handles: Vec<JoinHandle<()>>,
    /// Handle for shutting down threads
    shutdown: Arc<AtomicBool>,
}

impl PeerSamplingService {
    /// Create a new peer sampling service with provided parameters
    ///
    /// # Arguments
    ///
    /// * `config` - The parameters for the peer sampling protocol [PeerSamplingConfig]
    pub fn new(address: SocketAddr, config: PeerSamplingConfig) -> PeerSamplingService {
        PeerSamplingService {
            address,
            view: Arc::new(Mutex::new(View::new(address.to_string()))),
            config,
            thread_handles: Vec::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initializes service
    ///
    /// # Arguments
    ///
    /// * `initial_peer` - A closure returning the initial peer for starting the protocol
    pub fn init(&mut self, initial_peer: Box<dyn FnOnce() -> Option<Vec<Peer>>>, receiver: Receiver<PeerSamplingMessage>) {
        // get address of initial peer
        if let Some(initial_peers) = initial_peer() {
            let mut view = self.view.lock().unwrap();
            for peer in initial_peers {
                if peer.address() != &self.address.to_string() {
                    view.peers.push(peer);
                }
            }
        }

        // handle received messages
        let receiver_handle = self.start_receiver(receiver);
        self.thread_handles.push(receiver_handle);

        // start peer sampling
        let sampling_handle = self.start_sampling_activity();
        self.thread_handles.push(sampling_handle);

        log::info!("All activity threads were started");
    }

    /// Returns a random peer for the client application.
    /// The peer is pseudo-random peer from the set of all peers.
    /// The local view is built using [Gossip-Based Peer Sampling].
    pub fn get_peer(&mut self) -> Option<Peer> {
        self.view.lock().unwrap().get_peer()
    }

    /// Returns a copy of the list of peers in the node view
    pub fn peers(&self) -> Vec<Peer> {
        self.view.lock().unwrap()
            .peers.iter().map(|peer| peer.clone())
            .collect()
    }

    /// Stops the threads related to peer sampling activity
    pub fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        // request shutdown
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        {
            let mut view = self.view.lock().unwrap();
            view.peers.clear();
            view.queue.clear();
            crate::network::send(&view.host_address.parse()?, Box::new(NoopMessage))?;
        }
        // wait for termination
        let mut join_error = false;
        for handle in self.thread_handles.drain(..) {
            if let Err(e) = handle.join() {
                log::error!("Error joining thread: {:?}", e);
                join_error = true;
            }
        }
        log::info!("All activity threads were stopped");
        if join_error {
            Err("An error occurred during thread joining")?
        }
        else {
            Ok(())
        }
    }

    /// Builds the view to be exchanged with another peer
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration parameters
    /// * `view` - The current view
    fn build_buffer(address: String, config: &PeerSamplingConfig, view: &mut View) -> Vec<Peer> {
        let mut buffer = vec![ Peer::new(address) ];
        view.permute();
        view.move_oldest_to_end(config.healing_factor());
        buffer.append(&mut view.head(config.view_size()));
        buffer
    }

    /// Creates a thread for handling messages
    ///
    /// # Arguments
    ///
    /// * `receiver` - The channel used for receiving incoming messages
    fn start_receiver(&self, receiver: Receiver<PeerSamplingMessage>) -> JoinHandle<()>{
        let address = self.address.to_string();
        let sampling_config = self.config.clone();
        let view_arc = self.view.clone();
        std::thread::Builder::new().name(format!("{} - gbps receiver", &address)).spawn(move|| {
            log::info!("Started message handling thread");
            while let Ok(message) = receiver.recv() {
                log::debug!("Received: {:?}", message);
                let mut view = view_arc.lock().unwrap();
                if let MessageType::Request = message.message_type() {
                    if sampling_config.is_pull() {
                        let buffer = Self::build_buffer(address.clone(), &sampling_config, &mut view);
                        log::debug!("Built response buffer: {:?}", buffer);
                        if let Ok(remote_address) = message.sender().parse::<SocketAddr>() {
                            match crate::network::send(&remote_address, Box::new(PeerSamplingMessage::new_response(address.clone(), Some(buffer)))) {
                                Ok(written) => log::trace!("Buffer sent successfully ({} bytes)", written),
                                Err(e) => log::error!("Error sending buffer: {}", e),
                            }
                        }
                        else {
                            log::error!("Could not parse sender address {}", &message.sender());
                        }
                    }
                }

                if let Some(buffer) = message.view() {
                    view.select(sampling_config.view_size(), sampling_config.healing_factor(), sampling_config.swapping_factor(), &buffer);
                }
                else {
                    log::warn!("received a response with an empty buffer");
                }

                view.increase_age();
            }
            log::info!("Message handling thread exiting");
        }).unwrap()
    }

    /// Creates a thread that periodically executes the peer sampling
    fn start_sampling_activity(&self) -> JoinHandle<()> {
        let address = self.address.to_string();
        let config = self.config.clone();
        let view_arc = self.view.clone();
        let shutdown_requested = Arc::clone(&self.shutdown);
        std::thread::Builder::new().name(format!("{} - gbps sampling", address)).spawn(move || {
            log::info!("Started peer sampling thread");
            loop {
                // Compute time for sleep cycle
                let deviation =
                    if config.sampling_deviation() == 0 { 0 }
                    else { rand::thread_rng().gen_range(0, config.sampling_deviation()) };
                let sleep_time = config.sampling_period() + deviation;
                std::thread::sleep(std::time::Duration::from_millis(sleep_time));

                let mut view = view_arc.lock().unwrap();
                if let Some(peer) = view.select_peer() {
                    if config.is_push() {
                        let buffer = Self::build_buffer(address.clone(), &config, &mut view);
                        // send local view
                        if let Ok(remote_address) = &peer.address().parse::<SocketAddr>() {
                            match crate::network::send(remote_address, Box::new(PeerSamplingMessage::new_request(address.clone(), Some(buffer)))) {
                                Ok(written) => log::trace!("Buffer sent successfully ({} bytes)", written),
                                Err(e) => log::error!("Error sending buffer: {}", e),
                            }
                        }
                        else {
                            log::error!("Could not parse sender address {}", &peer.address());
                        }
                    }
                    else {
                        // send empty view to trigger response
                        if let Ok(remote_address) = &peer.address().parse::<SocketAddr>() {
                            match crate::network::send(remote_address, Box::new(PeerSamplingMessage::new_request(address.clone(), None))) {
                                Ok(written) => log::trace!("Empty view sent successfully ({} bytes)", written),
                                Err(e) => log::error!("Error sending empty view: {}", e),
                            }
                        }
                        else {
                            log::error!("Could not parse sender address {}", &peer.address());
                        }
                    }
                    view.increase_age();
                }
                else {
                    log::warn!("No peer found for sampling")
                }

                // check for shutdown request
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
            }

            log::info!("Peer sampling thread exiting");
        }).unwrap()
    }
}

/// The view at each node
struct View {
    /// The address of the node
    host_address: String,
    /// The list of peers in the node view
    peers: Vec<Peer>,
    /// The queue from which peer are retrieved for the application layer
    queue: VecDeque<Peer>,
}
impl View {
    /// Creates a new view with the node's address
    ///
    /// # Arguments
    ///
    /// * `address` - Addres of peer
    fn new(host_address: String) -> View {
        View {
            host_address,
            peers: vec![],
            queue: VecDeque::new(),
        }
    }

    /// Randomly select a peer for exchanging views at each cycle
    fn select_peer(&self) -> Option<Peer> {
        if self.peers.is_empty() {
            None
        }
        else {
            let selected_peer = rand::thread_rng().gen_range(0, self.peers.len());
            Some(self.peers[selected_peer].clone())
        }
    }

    /// Randomly reorder the current view
    fn permute(&mut self) {
        self.peers.shuffle(&mut rand::thread_rng());
    }

    /// Move the oldest peers to the end of the view if the size
    /// of the view is larger than the healing factor
    ///
    /// # Arguments
    ///
    /// * `h` - The number of peer that should be moved
    fn move_oldest_to_end(&mut self, h: usize) {
        if self.peers.len() > h {
            let mut h_oldest_peers = self.peers.clone();
            h_oldest_peers.sort_by_key(|peer| peer.age());
            h_oldest_peers.reverse();
            h_oldest_peers.truncate(h); //
            // (peers.len - h) at the beginning, h at the end
            let mut new_view_start = vec![];
            let mut new_view_end = vec![];
            for peer in &self.peers {
                if h_oldest_peers.contains(&peer) {
                    new_view_end.push(peer.clone());
                }
                else {
                    new_view_start.push(peer.clone());
                }
            }
            new_view_start.append(&mut new_view_end);
            std::mem::replace(&mut self.peers, new_view_start);
        }
    }

    /// Returns the peers at the beginning of the view
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    fn head(&self, c: usize) -> Vec<Peer> {
        let count = std::cmp::min(c / 2 - 1, self.peers.len());
        let mut head = Vec::new();
        for i in 0..count {
            head.push(self.peers[i].clone());
        }
        head
    }

    /// Increases by one the age of each peer in the view
    fn increase_age(&mut self) {
        for peer in self.peers.iter_mut() {
            peer.increment_age();
        }
    }

    /// Merge a view received received from a peer with the current view
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    /// * `h` - The healing parameter
    /// * `s` - The swap parameter
    /// * `buffer` - The view received
    fn select(&mut self, c:usize, h: usize, s: usize, buffer: &Vec<Peer>) {
        let my_address = self.host_address.clone();
        // Add received peers to current view, omitting the node's own address
        buffer.iter()
            .filter(|peer| peer.address() != my_address)
            .for_each(|peer| self.peers.push(peer.clone()));
        // Perform peer selection algorithm
        self.remove_duplicates();
        self.remove_old_items(c, h);
        self.remove_head(c, s);
        self.remove_at_random(c);
        // Update peer queue for application layer
        self.update_queue();
    }

    /// Removes duplicates peers from the view and keep the most recent one
    fn remove_duplicates(&mut self) {
        let mut unique_peers: HashSet<Peer> = HashSet::new();
        self.peers.iter().for_each(|peer| {
            if let Some(entry) = unique_peers.get(peer) {
                // duplicate peer, check age
                if peer.age() < entry.age() {
                    unique_peers.replace(peer.clone());
                }
            }
            else {
                // unique peer
                unique_peers.insert(peer.clone());
            }
        });
        let new_view = Vec::from_iter(unique_peers);
        std::mem::replace(&mut self.peers, new_view);
    }

    /// Removes the oldest items from the view based on the healing parameter
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    /// * `h` - The healing parameter
    fn remove_old_items(&mut self, c: usize, h: usize) {
        let min = if self.peers.len() > c { self.peers.len() - c } else { 0 };
        let removal_count = std::cmp::min(h, min);
        if removal_count > 0 {
            let mut kept_peers = self.peers.clone();
            kept_peers.sort_by_key(|peer| peer.age());
            kept_peers.truncate(kept_peers.len() - removal_count);
            let mut new_view = vec![];
            for peer in &self.peers {
                if kept_peers.contains(&peer) {
                    new_view.push(peer.clone());
                }
            }
            std::mem::replace(&mut self.peers, new_view);
        }
    }

    /// Removes peers at the beginning of the current view based on the swap parameter
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    /// * `s` - The swap parameter
    fn remove_head(&mut self, c: usize, s: usize) {
        let min = if self.peers.len() > c { self.peers.len() - c } else { 0 };
        let removal_count = std::cmp::min(s, min);
        self.peers.drain(0..removal_count);
    }

    /// Removes peers at random to match the view size parameter
    ///
    /// # Arguments
    ///
    /// * `c` - The size of the view
    fn remove_at_random(&mut self, c: usize) {
        if self.peers.len() > c {
            for _ in 0..(self.peers.len() - c) {
                let remove_index = rand::thread_rng().gen_range(0, self.peers.len());
                self.peers.remove(remove_index);
            }
        }
    }

    /// Update peer queue by adding peers that appeared in the view
    /// and removing those that were removed.
    fn update_queue(&mut self) {

        // compute index of removed peers
        let removed_peers = self.queue.iter().enumerate()
            .filter(|(_, peer)| !self.peers.contains(peer))
            .map(|(index, _)| index)
            .collect::<Vec<usize>>();

        // compute new peers
        let added_peers = self.peers.iter()
            .filter(|peer| !self.queue.contains(peer))
            .map(|peer| peer.to_owned())
            .collect::<Vec<Peer>>();

        // removed old peers by descending index
        removed_peers.iter().rev().for_each(|index| { self.queue.remove(*index); });

        // add new peers
        for peer in added_peers {
            self.queue.push_back(peer);
        }
    }

    /// Returns a random peer for use in the application layer.
    /// The peer is selected from the queue of newly added peers if available,
    /// otherwise at random from the view.
    pub fn get_peer(&mut self) -> Option<Peer> {
        if let Some(peer) = self.queue.pop_front() {
            Some(peer)
        }
        else {
            self.select_peer()
        }
    }
}
