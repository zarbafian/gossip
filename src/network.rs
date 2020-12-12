use std::net::{SocketAddr, TcpStream};
use std::io::{Write, Read};
use std::thread::JoinHandle;
use std::sync::Arc;
use std::error::Error;
use std::sync::mpsc::Sender;
use crate::message::{Message, PeerSamplingMessageParser, GossipMessageParser};
use crate::message::sampling::PeerSamplingMessage;
use crate::message::gossip::GossipMessage;

pub fn send(address: &SocketAddr, message: Box<dyn Message>) -> std::io::Result<usize> {
    let written = TcpStream::connect(address)?.write(&message.as_bytes())?;
    Ok(written)
}

pub fn listen(address: &SocketAddr, shutdown: Arc<std::sync::atomic::AtomicBool>, peer_sampling_sender: Sender<PeerSamplingMessage>, gossip_sender: Sender<GossipMessage>) -> std::io::Result<JoinHandle<()>> {

    let listener = std::net::TcpListener::bind(address)?;
    log::info!("Listener started at {}", address);
    Ok(std::thread::Builder::new().name(format!("{} - gossip listener", address)).spawn(move || {
        log::info!("Started listener thread");
        // TODO: handle hanging connections where peer connect but does not write
        for incoming_stream in listener.incoming() {

            // check for shutdown request
            if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                log::info!("Shutdown requested");
                break;
            }

            // TODO: handle in new thread or worker
            // handle request
            match incoming_stream {
                Ok(mut stream) => {
                    let mut buf = Vec::new();
                    match stream.read_to_end(&mut buf) {
                        Ok(read) => {
                            if read > 0 {
                                match handle_message(buf, &peer_sampling_sender, &gossip_sender) {
                                    Ok(()) => log::debug!("Message parsed successfully"),
                                    Err(e) => log::error!("{:?}", e),
                                }
                            }
                        },
                        Err(e) => log::error!("Error receiving data: {:?}", e),
                    }
                }
                Err(e) => log::warn!("Connection failed: {}", e),
            }
        }
        log::info!("Listener thread exiting");
    }).unwrap())
}

fn handle_message(buffer: Vec<u8>, peer_sampling_sender: &Sender<PeerSamplingMessage>, gossip_sender: &Sender<GossipMessage>) -> Result<(), Box<dyn Error>> {
    let protocol = buffer[0] & crate::message::MASK_PROTOCOL;
    match protocol {
        crate::message::MESSAGE_PROTOCOL_NOOP => Ok(()),
        crate::message::MESSAGE_PROTOCOL_SAMPLING => {
            let message = PeerSamplingMessageParser.parse(buffer)?;
            peer_sampling_sender.send(message)?;
            Ok(())
        },
        crate::message::MESSAGE_PROTOCOL_GOSSIP => {
            let message = GossipMessageParser.parse(buffer)?;
            gossip_sender.send(message)?;
            Ok(())
        },
        _ => Err(format!("Unknown protocol: {}", protocol))?
    }
}
