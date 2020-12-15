use std::thread::JoinHandle;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashMap;
use std::error::Error;
use rand::Rng;
use crate::config::{GossipConfig, UpdateExpirationValue};
use crate::PeerSamplingConfig;
use crate::sampling::PeerSamplingService;
use crate::message::gossip::{Update, UpdateHandler, HeaderMessage, ContentMessage};
use crate::message::{NoopMessage, MessageType};
use crate::peer::Peer;
use crate::message::sampling::PeerSamplingMessage;
use crate::monitor::MonitoringConfig;

pub struct GossipService<T> {
    /// Socket address of the node
    address: SocketAddr,
    /// Peer sampling service
    peer_sampling_service: Arc<Mutex<PeerSamplingService>>,
    /// Configuration for gossip
    gossip_config: GossipConfig,
    /// Shutdown requested flag
    shutdown: Arc<AtomicBool>,
    /// Thread handles
    activities: Vec<JoinHandle<()>>,
    /// Active updates
    active_updates: Arc<Mutex<HashMap<String, (Update, UpdateExpirationValue)>>>,
    /// Removed/expired updates
    removed_updates: Arc<Mutex<Vec<String>>>,
    /// Application callback for receiving new updates
    update_handler: Arc<Mutex<Option<Box<T>>>>,
    /// Monitoring configuration
    monitoring_config: MonitoringConfig,
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
    /// * `monitoring_config` - Configuration for monitoring, see [MonitoringConfig]
    pub fn new(address: SocketAddr, peer_sampling_config: PeerSamplingConfig, gossip_config: GossipConfig, monitoring_config: Option<MonitoringConfig>) -> GossipService<T> {
        let monitoring_config = monitoring_config.unwrap_or_default();
        GossipService{
            address,
            peer_sampling_service: Arc::new(Mutex::new(PeerSamplingService::new(address, peer_sampling_config, monitoring_config.clone()))),
            gossip_config,
            shutdown: Arc::new(AtomicBool::new(false)),
            activities: Vec::new(),
            active_updates: Arc::new(Mutex::new(HashMap::new())),
            removed_updates: Arc::new(Mutex::new(Vec::new())),
            update_handler: Arc::new(Mutex::new(None)),
            monitoring_config,
        }
    }

    pub fn address(&self) -> &SocketAddr {
        &self.address
    }

    /// Starts the gossip protocol and related threads
    ///
    /// # Arguments
    ///
    /// * `peer_sampling_init` - Closure for retrieving the address of the first peer to contact
    /// * `update_handler` - Application callback for receiving new updates
    pub fn start(&mut self, peer_sampling_init: Box<dyn FnOnce() -> Option<Vec<Peer>>>, update_handler: Box<T>) -> Result<(), Box<dyn Error>> {

        self.update_handler.lock().unwrap().replace(update_handler);

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
        let pull = self.gossip_config.is_pull();
        let address = self.address.to_string();
        let active_updates_arc = Arc::clone(&self.active_updates);
        let removed_updates_arc = Arc::clone(&self.removed_updates);
        let handle = std::thread::Builder::new().name(format!("{} - header receiver", address)).spawn(move|| {
            log::info!("Started message header handling thread");
            while let Ok(message) = receiver.recv() {

                if let Ok(sender_address) = message.sender().parse::<SocketAddr>() {

                    let active_updates = active_updates_arc.lock().unwrap();
                    let removed_updates = removed_updates_arc.lock().unwrap();

                    // Response with message headers
                    if pull && active_updates.len() > 0 && *message.message_type() == MessageType::Request {
                        let mut message = HeaderMessage::new_response(address.clone());
                        active_updates.iter()
                            .for_each(|(digest, _)| message.push(digest.to_owned()));
                        match crate::network::send(&sender_address, Box::new(message)) {
                            Ok(written) => log::trace!("Sent header response - {} bytes to {:?}", written, sender_address),
                            Err(e) => log::error!("Error sending header response: {:?}", e)
                        }
                    }

                    // Process received headers
                    let mut digests = HashMap::new();
                    message.messages().iter().for_each(|digest| {
                        if !active_updates.contains_key(digest) && !removed_updates.contains(digest){
                            log::debug!("New digest: {}", digest);
                            digests.insert(digest.to_owned(), vec![]);
                        }
                        else {
                            log::trace!("Duplicate digest: {}", digest);
                        }
                    });
                    if digests.len() > 0 {
                        let message = ContentMessage::new_request(address.clone(), digests);
                        match crate::network::send(&sender_address, Box::new(message)) {
                            Ok(written) => log::trace!("Sent content request - {} bytes to {:?}", written, sender_address),
                            Err(e) => log::error!("Error content request response: {:?}", e)
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
        let active_updates_arc = Arc::clone(&self.active_updates);
        let removed_updates_arc = Arc::clone(&self.removed_updates);
        let update_callback_arc = Arc::clone(&self.update_handler);
        let update_expiration = self.gossip_config.update_expiration().clone();
        let monitoring_config = self.monitoring_config.clone();
        let handle = std::thread::Builder::new().name(format!("{} - content receiver", address)).spawn(move|| {
            log::info!("Started message content handling thread");
            while let Ok(message) = receiver.recv() {

                match message.message_type() {
                    MessageType::Request => {
                        log::debug!("Received content request: {:?}", message.content());
                        if let Ok(peer_address) = message.sender().parse::<SocketAddr>() {
                            let active_updates = active_updates_arc.lock().unwrap();
                            if active_updates.len() > 0 {
                                let mut map = HashMap::new();
                                message.content().iter().for_each(|(digest, _)| {
                                    if let Some((update, _)) = active_updates.get(digest) {
                                        map.insert(digest.to_owned(), update.content().to_vec());
                                    }
                                });
                                let message = ContentMessage::new_response(address.clone(), map);
                                match crate::network::send(&peer_address, Box::new(message)) {
                                    Ok(written) => log::trace!("Sent content response - {} bytes to {:?}", written, peer_address),
                                    Err(e) => log::error!("Error content response: {:?}", e)
                                }
                            }
                        }
                    }
                    MessageType::Response => {
                        if message.content().len() > 0 {
                            let mut active_updates = active_updates_arc.lock().unwrap();
                            let removed_updates = removed_updates_arc.lock().unwrap();
                            message.content().iter().for_each(|(digest, content)| {
                                if !active_updates.contains_key(digest) && !removed_updates.contains(digest) {
                                    let update = Update::new(content.clone());
                                    if digest == update.digest() {
                                        log::info!("New update received: {}", update.digest());
                                        active_updates.insert(digest.to_owned(), (update, UpdateExpirationValue::new(update_expiration.clone())));
                                        let mutex = update_callback_arc.lock().unwrap();
                                        if let Some(callback) = mutex.as_ref() {
                                            let update_app = Update::new(content.clone());
                                            callback.on_update(update_app);
                                        }

                                    }
                                    else {
                                        log::warn!("Digests did not match: {} <> {}", digest, update.digest())
                                    }
                                }
                            });

                            // Monitoring
                            if monitoring_config.enabled() {
                                let updates = active_updates.iter().map(|(digest, _)| digest.to_owned()).collect::<Vec<String>>();
                                monitoring_config.send_update_data(address.clone(), updates);
                            }
                        }
                    }
                }
                //log::debug!("Received content of digest: {}", message.digest());
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
        let push = self.gossip_config.is_push();
        let node_address = self.address.to_string();
        let shutdown_requested = Arc::clone(&self.shutdown);
        let gossip_interval = self.gossip_config.gossip_interval();
        let gossip_deviation = self.gossip_config.gossip_deviation();
        let peer_sampling_arc = Arc::clone(&self.peer_sampling_service);
        let active_updates_arc = Arc::clone(&self.active_updates);
        let removed_updates_arc = Arc::clone(&self.removed_updates);
        let handle = std::thread::Builder::new().name(format!("{} - gossip activity", self.address().to_string())).spawn(move ||{
            log::info!("Gossip thread started");
            loop {
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                let sleep = gossip_interval + rand::thread_rng().gen_range(0, gossip_deviation);
                std::thread::sleep(std::time::Duration::from_millis(sleep));

                let mut peer_sampling_service = peer_sampling_arc.lock().unwrap();
                if let Some(peer) = peer_sampling_service.get_peer() {
                    if let Ok(peer_address) = peer.address().parse::<SocketAddr>() {
                        drop(peer_sampling_service);
                        let mut message = HeaderMessage::new_request(node_address.to_string());
                        if push {
                            // send active headers
                            let mut active_updates = active_updates_arc.lock().unwrap();
                            let mut removed_updates = removed_updates_arc.lock().unwrap();
                            let mut local_expired_updates = Vec::new();

                            if active_updates.len() > 0 {
                                active_updates.iter_mut()
                                    .for_each(|(digest, (_, expiration_value))| {
                                        // gossip the update
                                        message.push(digest.to_owned());
                                        // check for expiration
                                        expiration_value.increase_age();
                                        if expiration_value.has_expired() {
                                            log::info!("Update expired: {}", digest);
                                            local_expired_updates.push(digest.clone());
                                        }
                                    });

                                local_expired_updates.iter().for_each(|digest| {
                                    active_updates.remove(digest);
                                    removed_updates.push(digest.to_owned());
                                });
                            }
                        }
                        else {
                            // send empty headers to trigger response
                        }

                        log::debug!("Will send header request with {:?}", message.messages());

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
        let mut active_updates = self.active_updates.lock().unwrap();
        let removed_updates = self.removed_updates.lock().unwrap();
        if active_updates.contains_key(update.digest()) {
            Err("Message already active")?
        }
        else if removed_updates.contains(update.digest()) {
            Err("Submitted expired message")?
        }
        else {
            log::info!("New update for submission: {}", update.digest());
            active_updates.insert(update.digest().to_owned(), (update, UpdateExpirationValue::new(self.gossip_config.update_expiration().clone())));
            Ok(())
        }
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

        if error {
            Err("toto")?
        }
        else {
            Ok(())
        }
    }
}

