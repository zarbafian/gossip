use std::thread::JoinHandle;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::mpsc::{Sender, Receiver};
use crate::config::GossipConfig;
use crate::PeerSamplingConfig;
use crate::sampling::PeerSamplingService;
use crate::message::gossip::GossipMessage;
use crate::message::NoopMessage;
use crate::peer::Peer;
use crate::message::sampling::PeerSamplingMessage;

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
        }
    }

    /// Start gossiping-related activities
    pub fn start(&mut self, peer_sampling_init: Box<dyn FnOnce() -> Option<Vec<Peer>>>) -> Result<(), Box<dyn Error>> {
        // start peer sampling
        let (tx_sampling, rx_sampling) = std::sync::mpsc::channel::<PeerSamplingMessage>();
        {
            self.peer_sampling_service.lock().unwrap().init(peer_sampling_init, rx_sampling);
        }
        let (tx_gossip, rx_gossip) = std::sync::mpsc::channel::<GossipMessage>();

        // start gossip message handler
        self.start_gossip_receiver(rx_gossip).expect("Error starting message handler for gossip message");
        // start TCP listener
        self.start_network_listener(tx_sampling, tx_gossip).expect(&format!("Error setting up listener at {:?}", self.address));
        // start gossiping
        self.start_gossip_activity().expect("Error starting gossip activity");
        Ok(())
    }

    fn start_gossip_receiver(&mut self, receiver: Receiver<GossipMessage>) -> Result<(), Box<dyn Error>> {
        let address = self.address.to_string();
        let handle = std::thread::Builder::new().name(format!("{} - gossip receiver", address)).spawn(move|| {
            log::info!("Started message handling thread");
            while let Ok(message) = receiver.recv() {
                log::debug!("Received: {:?}", message);
            }
            log::info!("Message handling thread exiting");
        }).unwrap();
        self.activities.push(handle);
        Ok(())
    }

    fn start_network_listener(&mut self, peer_sampling_sender: Sender<PeerSamplingMessage>, gossip_sender: Sender<GossipMessage>) -> Result<(), Box<dyn Error>> {
        let handle = crate::network::listen(self.gossip_config.address(),  Arc::clone(&self.shutdown), peer_sampling_sender, gossip_sender)?;
        self.activities.push(handle);
        Ok(())
    }

    fn start_gossip_activity(&mut self) -> Result<(), Box<dyn Error>> {
        let shutdown_requested = Arc::clone(&self.shutdown);
        let gossip_interval = self.gossip_config.gossip_interval();
        let peer_sampling_arc = Arc::clone(&self.peer_sampling_service);
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
                        let message = GossipMessage::new();
                        match crate::network::send(&address, Box::new(message)) {
                            Ok(written) => log::debug!("sent {} bytes to {:?}", written, address),
                            Err(e) => log::error!("Error sending message: {:?}", e)
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

    pub fn submit(&self, bytes: Vec<u8>) {

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

