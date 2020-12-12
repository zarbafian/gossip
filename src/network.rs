use std::net::{SocketAddr, TcpStream};
use std::io::{Write, Read};
use std::thread::JoinHandle;
use std::sync::Arc;
use std::error::Error;
use serde::Serialize;
use std::sync::mpsc::Sender;
use crate::message::Message;
use crate::message::sampling::PeerSamplingMessage;
use crate::message::gossip::{ContentMessage, HeaderMessage};

pub fn send<M>(address: &SocketAddr, message: Box<M>) -> Result<usize, Box<dyn Error>>
where M: Message + Serialize
{
    match message.as_bytes() {
        Ok(mut bytes) => {
            let mut buffer = vec![message.protocol()];
            buffer.append(&mut bytes);
            let written = TcpStream::connect(address)?.write(&buffer)?;
            Ok(written)
        }
        Err(e) => {
            log::error!("Could not serialize message");
            Err(e)?
        }
    }
}

pub fn listen(address: &SocketAddr, shutdown: Arc<std::sync::atomic::AtomicBool>, peer_sampling_sender: Sender<PeerSamplingMessage>, gossip_sender: Sender<ContentMessage>) -> std::io::Result<JoinHandle<()>> {

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

fn handle_message(buffer: Vec<u8>, peer_sampling_sender: &Sender<PeerSamplingMessage>, header_sender: &Sender<ContentMessage>) -> Result<(), Box<dyn Error>> {
    let protocol = buffer[0] & crate::message::MASK_MESSAGE_PROTOCOL;
    match protocol {
        crate::message::MESSAGE_PROTOCOL_NOOP_MESSAGE => Ok(()),
        crate::message::MESSAGE_PROTOCOL_SAMPLING_MESSAGE => {
            let message = PeerSamplingMessage::from_bytes(&buffer[1..])?;
            peer_sampling_sender.send(message)?;
            Ok(())
        }
        crate::message::MESSAGE_PROTOCOL_CONTENT_MESSAGE => {
            let message = ContentMessage::from_bytes(&buffer[1..])?;
            header_sender.send(message)?;
            Ok(())
        }
        crate::message::MESSAGE_PROTOCOL_HEADER_MESSAGE => {
            let message = HeaderMessage::from_bytes(&buffer[1..])?;
            // TODO
            // TODO
            // TODO
            //header_sender.send(message)?;
            Ok(())
        }
        _ => Err(format!("Unknown protocol: {}", protocol))?
    }
}
