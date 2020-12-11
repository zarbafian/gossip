use std::thread::JoinHandle;
use crate::config::GossipConfig;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::error::Error;

pub struct GossipService {
    config: GossipConfig,
    shutdown: Arc<AtomicBool>,
    activities: Vec<JoinHandle<()>>,
}

impl GossipService {
    pub fn new(config: GossipConfig) -> GossipService {
        GossipService{
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
            activities: Vec::new(),
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        self.start_gossip_activity()?;
        Ok(())
    }

    fn start_network_listener(&mut self) -> Result<(), Box<dyn Error>> {

        let listener = std::net::TcpListener::bind(self.config.address())?;
        log::info!("Listener started at {}", self.config.address().to_string());

        // shutdown flag
        let shutdown_requested = Arc::clone(&self.shutdown);

        let handle = std::thread::Builder::new().name(format!("{} - network listener", self.config.address())).spawn(move || {
            log::info!("Started listener thread");
            // TOD: handle hanging connections where peer connect but does not write
            for incoming_stream in listener.incoming() {

                // check for shutdown request
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    log::info!("Shutdown requested");
                    break;
                }

                // TODO: handle in new thread or worker
                // handle request
                match incoming_stream {
                    Ok(mut stream) => {
                        log::debug!("data received");
                        /*
                        if let Err(e) = handle_message(&mut stream, &sender) {
                            log::error!("Error processing request: {}", e);
                        }
                         */
                    }
                    Err(e) => log::warn!("Connection failed: {}", e),
                }
            }
            log::info!("Listener thread exiting");
        }).unwrap();

        self.activities.push(handle);

        Ok(())
    }

    fn start_gossip_activity(&mut self) -> Result<(), Box<dyn Error>> {
        let shutdown_requested = Arc::clone(&self.shutdown);
        let gossip_interval = self.config.gossip_interval();
        let handle = std::thread::Builder::new().name(format!("{} - gossip activity", self.config.address().to_string())).spawn(move ||{
            log::info!("Gossip thread started");
            loop {
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(gossip_interval));

                log::info!("Boom!");
            }
            log::info!("Gossip thread exiting");
        }).unwrap();

        self.activities.push(handle);

        Ok(())
    }

    pub fn submit(&self, bytes: Vec<u8>) {

    }

    pub fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        log::info!("Shutdown requested");
        let mut error = false;
        self.activities.drain(..).for_each(move|handle| {
            if let Err(e) = handle.join() {
                error = true;
            }
        });
        log::info!("All thread terminated");
        if error {
            Err("toto")?
        }
        else {
            Ok(())
        }
    }
}

