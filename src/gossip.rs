use std::thread::JoinHandle;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use std::net::SocketAddr;
use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashMap;
use std::error::Error;
use rand::Rng;
use crate::config::GossipConfig;
use crate::PeerSamplingConfig;
use crate::sampling::PeerSamplingService;
use crate::update::{Update, UpdateHandler, UpdateDecorator};
use crate::message::gossip::{HeaderMessage, ContentMessage};
use crate::message::{NoopMessage, MessageType};
use crate::peer::Peer;
use crate::message::sampling::PeerSamplingMessage;

/// The gossip service
pub struct GossipService<T> {
    /// Socket address of the node
    address: SocketAddr,
    /// Peer sampling service
    peer_sampling_service: Arc<Mutex<PeerSamplingService>>,
    /// Configuration for gossip
    gossip_config: Arc<GossipConfig>,
    /// Shutdown requested flag
    shutdown: Arc<AtomicBool>,
    /// Thread handles
    activities: Vec<JoinHandle<()>>,
    /// Active and expired updates
    updates: Arc<RwLock<UpdateDecorator>>,
    /// Application callback for receiving new updates
    update_handler: Arc<Mutex<Option<Box<T>>>>,
}

impl<T> GossipService<T>
where T: UpdateHandler + 'static + Send
{
    /// Creates a new gossiping service
    ///
    /// # Arguments
    ///
    /// * `address` - Socket address of the node
    /// * `peer_sampling_config` - Configuration for peer sampling, see [PeerSamplingConfig]
    /// * `gossip_config` - Configuration for gossiping, see [GossipConfig]
    pub fn new(address: SocketAddr, peer_sampling_config: PeerSamplingConfig, gossip_config: GossipConfig) -> GossipService<T> {
        GossipService{
            address,
            peer_sampling_service: Arc::new(Mutex::new(PeerSamplingService::new(address, peer_sampling_config))),
            updates: Arc::new(RwLock::new(UpdateDecorator::new(gossip_config.update_expiration().clone()))),
            gossip_config: Arc::new(gossip_config),
            shutdown: Arc::new(AtomicBool::new(false)),
            activities: Vec::new(),
            update_handler: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a gossip service with default configurations
    ///
    /// # Arguments
    ///
    /// * `address` - Socket address of the node
    pub fn new_with_defaults(address: SocketAddr) -> Self {
        Self::new(address, PeerSamplingConfig::default(), GossipConfig::default())
    }

    /// Returns the node address
    pub fn address(&self) -> &SocketAddr {
        &self.address
    }

    /// Returns a list of the node's peer
    pub fn peers(&self) -> Vec<Peer> {
        self.peer_sampling_service.lock().unwrap().peers()
    }

    /// Starts the gossip protocol and related threads
    ///
    /// # Arguments
    ///
    /// * `peer_sampling_init` - Closure for retrieving the address of the first peer to contact
    /// * `update_handler` - Application callback for receiving new updates
    pub fn start(&mut self, peer_sampling_init: Box<dyn FnOnce() -> Option<Vec<Peer>>>, update_handler: Box<T>) -> Result<(), Box<dyn Error>> {

        self.update_handler.lock().unwrap().replace(update_handler);

        // message receiver for peer sampling messages
        let (tx_sampling, rx_sampling) = std::sync::mpsc::channel::<PeerSamplingMessage>();
        {
            // start peer sampling
            self.peer_sampling_service.lock().unwrap().init(peer_sampling_init, rx_sampling);
        }
        // message receiver for header messages
        let (tx_header, rx_header) = std::sync::mpsc::channel::<HeaderMessage>();
        // message receiver for content messages
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
        let gossip_config_arc = Arc::clone(&self.gossip_config);
        let address = self.address.to_string();
        let updates_arc = Arc::clone(&self.updates);
        let handle = std::thread::Builder::new().name(format!("{} - header receiver", address)).spawn(move|| {
            log::info!("Started message header handling thread");
            while let Ok(message) = receiver.recv() {

                if let Ok(sender_address) = message.sender().parse::<SocketAddr>() {

                    let updates = updates_arc.read().unwrap();

                    // Response with message headers if pull is enabled
                    if gossip_config_arc.is_pull() && updates.active_count() > 0 && *message.message_type() == MessageType::Request {
                        let mut response = HeaderMessage::new_response(address.clone());
                        response.set_headers(updates.active_headers());
                        match crate::network::send(&sender_address, Box::new(response)) {
                            Ok(written) => log::trace!("Sent header response - {} bytes to {:?}", written, sender_address),
                            Err(e) => log::error!("Error sending header response: {:?}", e)
                        }
                    }

                    // Process message if (request and push enabled) or (response and pull enabled)
                    if *message.message_type() == MessageType::Request && gossip_config_arc.is_push() || *message.message_type() == MessageType::Response && gossip_config_arc.is_pull() {

                        let mut new_digests = HashMap::new();
                        message.headers().iter().for_each(|digest| {
                            if updates.is_new(digest) {
                                log::debug!("New digest: {}", digest);
                                new_digests.insert(digest.to_owned(), vec![]);
                            }
                            else {
                                log::trace!("Duplicate digest: {}", digest);
                            }
                        });
                        if new_digests.len() > 0 {
                            let content_request = ContentMessage::new_request(address.clone(), new_digests);
                            match crate::network::send(&sender_address, Box::new(content_request)) {
                                Ok(written) => log::trace!("Sent content request - {} bytes to {:?}", written, sender_address),
                                Err(e) => log::error!("Error content request response: {:?}", e)
                            }
                        }
                    }
                }
                else {
                    log::error!("Could not parse sender address {}", message.sender());
                }
            }
            log::info!("Message header handling thread exiting");
        }).unwrap();
        self.activities.push(handle);
        Ok(())
    }

    fn start_message_content_handler(&mut self, receiver: Receiver<ContentMessage>) -> Result<(), Box<dyn Error>> {
        let address = self.address.to_string();
        let updates_arc = Arc::clone(&self.updates);
        let update_callback_arc = Arc::clone(&self.update_handler);
        let handle = std::thread::Builder::new().name(format!("{} - content receiver", address)).spawn(move|| {
            log::info!("Started message content handling thread");
            while let Ok(message) = receiver.recv() {

                match message.message_type() {
                    MessageType::Request => {
                        if let Ok(peer_address) = message.sender().parse::<SocketAddr>() {
                            let updates = updates_arc.read().unwrap();
                            let mut requested_updates = HashMap::new();
                            for (digest, _) in message.content() {
                                if let Some(update) = updates.get_update(&digest) {
                                    requested_updates.insert(digest.to_owned(), update.content().to_vec());
                                }
                            }
                            if requested_updates.len() > 0{
                                let response = ContentMessage::new_response(address.clone(), requested_updates);
                                match crate::network::send(&peer_address, Box::new(response)) {
                                    Ok(written) => log::trace!("Sent content response - {} bytes to {:?}", written, peer_address),
                                    Err(e) => log::error!("Error content response: {:?}", e)
                                }
                            }
                        }
                    }
                    MessageType::Response => {
                        if message.len() > 0 {
                            let mut updates = updates_arc.write().unwrap();
                            for (digest, content) in message.content() {
                                if updates.is_new(&digest) {
                                    let update = Update::new(content.clone());
                                    if digest == *update.digest() {
                                        log::info!("New update received: {}", update.digest());
                                        match updates.insert_update(update) {
                                            Ok(()) => {
                                                // insert OK, notify update handler
                                                let mutex = update_callback_arc.lock().unwrap();
                                                if let Some(callback) = mutex.as_ref() {
                                                    let update = Update::new(content);
                                                    callback.on_update(update);
                                                }
                                                else {
                                                    log::warn!("No update handler found");
                                                }
                                            },
                                            Err(e) => log::error!("Could not add update: {:?}", e),
                                        }
                                    }
                                    else {
                                        log::warn!("Digests did not match: {} <> {}", digest, update.digest());
                                    }
                                }
                            }
                            updates.clear_expired();
                        }
                    }
                }
            }
        }).unwrap();
        self.activities.push(handle);
        Ok(())
    }

    fn start_network_listener(&mut self, peer_sampling_sender: Sender<PeerSamplingMessage>, header_sender: Sender<HeaderMessage>, content_sender: Sender<ContentMessage>) -> Result<(), Box<dyn Error>> {
        let handle = crate::network::listen(self.address(), Arc::clone(&self.shutdown), peer_sampling_sender, header_sender, content_sender)?;
        self.activities.push(handle);
        Ok(())
    }

    fn start_gossip_activity(&mut self) -> Result<(), Box<dyn Error>> {
        let gossip_config_arc = Arc::clone(&self.gossip_config);
        let node_address = self.address.to_string();
        let shutdown_requested = Arc::clone(&self.shutdown);
        let peer_sampling_arc = Arc::clone(&self.peer_sampling_service);
        let updates_arc = Arc::clone(&self.updates);
        let handle = std::thread::Builder::new().name(format!("{} - gossip activity", self.address().to_string())).spawn(move ||{
            log::info!("Gossip thread started");
            loop {
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                let deviation =
                    if gossip_config_arc.gossip_deviation() == 0 { 0 }
                    else { rand::thread_rng().gen_range(0, gossip_config_arc.gossip_deviation()) };
                let sleep = gossip_config_arc.gossip_period() + deviation;
                std::thread::sleep(std::time::Duration::from_millis(sleep));

                let mut peer_sampling_service = peer_sampling_arc.lock().unwrap();
                if let Some(peer) = peer_sampling_service.get_peer() {
                    if let Ok(peer_address) = peer.address().parse::<SocketAddr>() {
                        drop(peer_sampling_service);
                        let mut message = HeaderMessage::new_request(node_address.to_string());
                        if gossip_config_arc.is_push() {
                            // send active headers
                            let mut updates = updates_arc.write().unwrap();

                            if updates.active_count() > 0 {
                                let active_headers = updates.active_headers_for_push();
                                message.set_headers(active_headers);
                                updates.clear_expired();
                            }
                        }
                        else {
                            // will send empty headers to trigger response
                        }

                        log::debug!("Will send header request with {:?}", message.headers());

                        // TODO: check expiration after sending
                        match crate::network::send(&peer_address, Box::new(message)) {
                            Ok(written) => log::trace!("Sent header request - {} bytes to {:?}", written, peer_address),
                            Err(e) => log::error!("Error sending header request: {:?}", e)
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
    /// * `bytes` - Content of the message
    pub fn submit(&self, bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let update = Update::new(bytes);
        let mut updates = self.updates.write().unwrap();
        if updates.is_new(update.digest()) {
            log::info!("New update for submission: {}", update.digest());
            updates.insert_update(update)?;
            Ok(())
        }
        else {
            Err("Message already active or expired")?
        }
    }

    // for testing
    pub fn is_active(&self, bytes: Vec<u8>) -> bool {
        self.updates.read().unwrap().is_active(Update::new(bytes).digest())
    }
    pub fn is_expired(&self, bytes: Vec<u8>) -> bool {
        self.updates.read().unwrap().is_expired(Update::new(bytes).digest())
    }

    /// Terminates the gossip protocol and related threads
    pub fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        self.update_handler.lock().unwrap().take();
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        log::info!("Shutdown requested");
        if let Ok(_) = crate::network::send(self.address(), Box::new(NoopMessage)) {
            // shutdown request sent
        }
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

        // clear updates
        self.updates.write().unwrap().clear();

        if error {
            Err("Error occurred during shutdown")?
        }
        else {
            Ok(())
        }
    }
}

