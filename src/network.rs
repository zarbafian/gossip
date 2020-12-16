use std::net::{SocketAddr, TcpStream};
use std::io::{Write, Read};
use std::thread::JoinHandle;
use std::sync::Arc;
use std::error::Error;
use serde::Serialize;
use std::sync::mpsc::Sender;
use crate::message::{Message, MASK_MESSAGE_PROTOCOL, MESSAGE_PROTOCOL_SAMPLING_MESSAGE, MESSAGE_PROTOCOL_HEADER_MESSAGE, MESSAGE_PROTOCOL_CONTENT_MESSAGE, MESSAGE_PROTOCOL_NOOP_MESSAGE};
use crate::message::sampling::PeerSamplingMessage;
use crate::message::gossip::{HeaderMessage, ContentMessage};

/// Sends a message to the specified address
///
/// # Arguments
///
/// * `address` - Address of the recipient
/// * `message` - Message implementing the [Message] trait
pub fn send<M>(address: &SocketAddr, message: Box<M>) -> Result<usize, Box<dyn Error>>
where M: Message + Serialize
{
    match message.as_bytes() {
        Ok(mut bytes) => {
            // insert protocol byte for deserialization
            bytes.insert(0, message.protocol());
            let written = TcpStream::connect(address)?.write(&bytes)?;
            Ok(written)
        }
        Err(e) => {
            log::error!("Could not serialize message");
            Err(e)?
        }
    }
}

/// Starts listening to TCP connections
///
/// # Arguments
///
/// * `address` - Bind address
/// * `shutdown` - Flag used to check for a shutdown request
/// * `peer_sampling_sender` - Used to dispatch peer sampling messages
/// * `header_sender` - Used to dispatch gossip header messages
/// * `content_sender` - Used to dispatch gossip content messages
pub fn listen(address: &SocketAddr, shutdown: Arc<std::sync::atomic::AtomicBool>, peer_sampling_sender: Sender<PeerSamplingMessage>, header_sender: Sender<HeaderMessage>, content_sender: Sender<ContentMessage>) -> std::io::Result<JoinHandle<()>> {

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
                                match handle_message(buf, &peer_sampling_sender, &header_sender, &content_sender) {
                                    Ok(()) => log::trace!("Message parsed successfully"),
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

fn handle_message(buffer: Vec<u8>, peer_sampling_sender: &Sender<PeerSamplingMessage>, header_sender: &Sender<HeaderMessage>, content_sender: &Sender<ContentMessage>) -> Result<(), Box<dyn Error>> {
    let protocol = buffer[0] & MASK_MESSAGE_PROTOCOL;
    match protocol {
        MESSAGE_PROTOCOL_NOOP_MESSAGE => Ok(()),
        MESSAGE_PROTOCOL_SAMPLING_MESSAGE => {
            let message = PeerSamplingMessage::from_bytes(&buffer[1..])?;
            peer_sampling_sender.send(message)?;
            Ok(())
        }
        MESSAGE_PROTOCOL_CONTENT_MESSAGE => {
            let message = ContentMessage::from_bytes(&buffer[1..])?;
            content_sender.send(message)?;
            Ok(())
        }
        MESSAGE_PROTOCOL_HEADER_MESSAGE => {
            let message = HeaderMessage::from_bytes(&buffer[1..])?;
            header_sender.send(message)?;
            Ok(())
        }
        _ => Err(format!("Unknown protocol: {}", protocol))?
    }
}
