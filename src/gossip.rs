use std::thread::JoinHandle;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::sync::mpsc::{Sender, Receiver};
use crate::config::GossipConfig;
use crate::PeerSamplingConfig;
use crate::sampling::PeerSamplingService;
use crate::message::gossip::{ContentMessage, HeaderMessage};
use crate::message::NoopMessage;
use crate::peer::Peer;
use crate::message::sampling::PeerSamplingMessage;
use std::collections::HashMap;
use std::error::Error;

pub struct GossipService {
    /// Address of node
    address: SocketAddr,
    /// Peer sampling service
    peer_sampling_service: Arc<Mutex<PeerSamplingService>>,
    /// Configuration for gossip
    gossip_config: GossipConfig,
    /// Shutdown requested flag
    shutdown: Arc<AtomicBool>,
    /// Thread handles
    activities: Vec<JoinHandle<()>>,

    // TODO
    active_messages: Arc<Mutex<HashMap<String, String>>>
    //messages: Arc<Mutex<HashMap<String, Conten>>>
}

impl GossipService {
    /// Creates a new gossiping service
    ///
    /// # Arguments
    ///
    /// * `peer_sampling_config` - Configuration for peer sampling
    /// * `gossip_config` - Configuration for gossiping
    pub fn new(address: SocketAddr, peer_sampling_config: PeerSamplingConfig, gossip_config: GossipConfig) -> GossipService {
        GossipService{
            address,
            peer_sampling_service: Arc::new(Mutex::new(PeerSamplingService::new(address, peer_sampling_config))),
            gossip_config,
            shutdown: Arc::new(AtomicBool::new(false)),
            activities: Vec::new(),
            active_messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start gossiping-related activities
    pub fn start(&mut self, peer_sampling_init: Box<dyn FnOnce() -> Option<Vec<Peer>>>) -> Result<(), Box<dyn Error>> {
        // start peer sampling
        let (tx_sampling, rx_sampling) = std::sync::mpsc::channel::<PeerSamplingMessage>();
        {
            self.peer_sampling_service.lock().unwrap().init(peer_sampling_init, rx_sampling);
        }
        let (tx_header, rx_header) = std::sync::mpsc::channel::<HeaderMessage>();
        let (tx_content, rx_content) = std::sync::mpsc::channel::<ContentMessage>();

        // start message header handler
        self.start_message_header_handler(rx_header).expect("Error starting message header handler");
        // start message content handler
        self.start_message_content_handler(rx_content).expect("Error starting message content handler");
        // start TCP listener
        self.start_network_listener(tx_sampling, tx_header, tx_content).expect(&format!("Error setting up listener at {:?}", self.address));
        // start gossiping
        self.start_gossip_activity().expect("Error starting gossip activity");
        Ok(())
    }

    fn start_message_header_handler(&mut self, receiver: Receiver<HeaderMessage>) -> Result<(), Box<dyn Error>> {
        let address = self.address.to_string();
        let active_messages_arc = Arc::clone(&self.active_messages);
        let handle = std::thread::Builder::new().name(format!("{} - header receiver", address)).spawn(move|| {
            log::info!("Started message header handling thread");
            while let Ok(message) = receiver.recv() {
                let mut active_messages = active_messages_arc.lock().unwrap();
                // TODO: check also digest
                message.messages().iter().for_each(|(id, digest)| {
                    log::debug!("Received: {} -> {}", id, digest);
                    if !active_messages.contains_key(id) {
                        active_messages.insert(id.to_owned(), digest.to_owned());
                    }
                });
            }
            log::info!("Message header handling thread exiting");
        }).unwrap();
        self.activities.push(handle);
        Ok(())
    }
    fn start_message_content_handler(&mut self, receiver: Receiver<ContentMessage>) -> Result<(), Box<dyn Error>> {
        let address = self.address.to_string();
        let handle = std::thread::Builder::new().name(format!("{} - content receiver", address)).spawn(move|| {
            log::info!("Started message content handling thread");
            while let Ok(message) = receiver.recv() {
                log::debug!("Received: {} -> {}", message.id(), String::from_utf8(message.content().to_vec()).unwrap());
            }
            log::info!("Message content handling thread exiting");
        }).unwrap();
        self.activities.push(handle);
        Ok(())
    }

    fn start_network_listener(&mut self, peer_sampling_sender: Sender<PeerSamplingMessage>, header_sender: Sender<HeaderMessage>, content_sender: Sender<ContentMessage>) -> Result<(), Box<dyn Error>> {
        let handle = crate::network::listen(self.gossip_config.address(), Arc::clone(&self.shutdown), peer_sampling_sender, header_sender, content_sender)?;
        self.activities.push(handle);
        Ok(())
    }

    fn start_gossip_activity(&mut self) -> Result<(), Box<dyn Error>> {
        let shutdown_requested = Arc::clone(&self.shutdown);
        let gossip_interval = self.gossip_config.gossip_interval();
        let peer_sampling_arc = Arc::clone(&self.peer_sampling_service);
        let active_messages_arc = Arc::clone(&self.active_messages);
        let handle = std::thread::Builder::new().name(format!("{} - gossip activity", self.gossip_config.address().to_string())).spawn(move ||{
            log::info!("Gossip thread started");
            loop {
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(gossip_interval));

                let mut peer_sampling_service = peer_sampling_arc.lock().unwrap();
                if let Some(peer) = peer_sampling_service.get_peer() {
                    if let Ok(address) = peer.address().parse::<SocketAddr>() {
                        drop(peer_sampling_service);
                        let active_messages = active_messages_arc.lock().unwrap();
                        if active_messages.len() > 0 {
                            let mut message = HeaderMessage::new();
                            active_messages.iter()
                                .for_each(|(id, digest)| message.push(id.to_owned(), digest.to_owned()));
                            match crate::network::send(&address, Box::new(message)) {
                                Ok(written) => log::debug!("sent {} bytes to {:?}", written, address),
                                Err(e) => log::error!("Error sending message: {:?}", e)
                            }
                        }
                    }
                }
                else {
                    log::warn!("No peer found for gossiping");
                }
            }
            log::info!("Gossip thread exiting");
        }).unwrap();

        self.activities.push(handle);

        Ok(())
    }

    /// Submits a message for broadcast by the gossip protocol
    ///
    /// # Arguments
    ///
    /// `message_id` - A unique identifier for the message
    /// `bytes` - Content of the message.
    pub fn submit(&self, message_id: String, bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut active_messages = self.active_messages.lock().unwrap();
        if active_messages.contains_key(&message_id) {
            Err("Message already active")?
        }
        else {
            let message = ContentMessage::new(message_id.clone(), bytes);
            active_messages.insert(message_id.clone(), message.digest());
            // TODO: store actual content
            Ok(())
        }
    }

    /// Terminate gossiping-related activities
    pub fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        log::info!("Shutdown requested");
        crate::network::send(self.gossip_config.address(), Box::new(NoopMessage));
        let mut error = false;
        self.activities.drain(..).for_each(move|handle| {
            if let Err(e) = handle.join() {
                log::error!("Error during thread join: {:?}", e);
                error = true;
            }
        });
        log::info!("All thread terminated");

        // terminate peer sampling
        self.peer_sampling_service.lock().unwrap().shutdown()?;

        if error {
            Err("toto")?
        }
        else {
            Ok(())
        }
    }
}

