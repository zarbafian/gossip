use std::error::Error;
use std::fmt::Debug;
use crate::peer::Peer;
use crate::message::Message;

// TODO: Remove
const MSG_TYPE_REQ: u8 = 0x80; // 0b1000000
const MSG_TYPE_RESP: u8 = 0x00;
const MASK_MSG_TYPE: u8 = 0x80; // 0b1000000

/// The message type
#[derive(Debug)]
pub enum MessageType {
    Request,
    Response
}

/// A peer sampling protocol message
#[derive(Debug)]
pub struct PeerSamplingMessage {
    /// Address of the sender
    sender: String,
    /// Type of the message
    message_type: MessageType,
    /// The view of the sender
    view: Option<Vec<Peer>>,
}

impl PeerSamplingMessage {
    /// Creates a new message of type [MessageType::Request] containing a view
    pub fn new_request(sender: String, view: Option<Vec<Peer>>) -> Self {
        Self::new(sender, MessageType::Request, view)
    }

    /// Creates a new message of type [MessageType::Response] containing a view
    pub fn new_response(sender: String, view: Option<Vec<Peer>>) -> Self {
        Self::new(sender, MessageType::Response, view)
    }

    fn new(sender: String, message_type: MessageType, view: Option<Vec<Peer>>) -> Self {
        Self {
            sender,
            message_type,
            view
        }
    }

    /// Returns the message sender
    pub fn sender(&self) -> &str {
        &self.sender
    }

    /// Returns the message type
    pub fn message_type(&self) -> &MessageType {
        &self.message_type
    }

    /// Returns the view contained in the message
    pub fn view(&self) -> &Option<Vec<Peer>> {
        &self.view
    }

    /// Deserializes a message from bytes
    ///
    /// # Arguments
    ///
    /// * `bytes` - A message serialized as bytes
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Box<dyn Error>> {

        // message type(1) + sender size(1) + one byte for sender(>=1) + view size(1)
        if bytes.len() < 4 {
            Err("invalid message")?
        }

        // message type
        let message_type = match bytes[0] & MASK_MSG_TYPE {
            MSG_TYPE_REQ => MessageType::Request,
            MSG_TYPE_RESP => MessageType::Response,
            _ => return Err("invalid message type")?,
        };

        // sender
        let sender_size = bytes[1] as usize;
        // message type(1) + sender size(1) + sender(sender_size) + view size(>=1)
        if bytes.len() < 3 + sender_size {
            Err("invalid message")?
        }
        let sender = String::from_utf8(bytes[2..2+sender_size].to_vec())?;

        // view size
        let view_size = bytes[2+sender_size];
        // message type(1) + sender size(1) + sender(sender_size) + view size(2 * view_size)
        if bytes.len() < (2 + sender_size + 2 * view_size as usize) {
            Err("invalid message")?
        }
        if view_size > 0 {
            let mut index = 3+sender_size;
            let mut peers = vec![];
            for _ in 0..view_size {
                let peer_length = bytes[index] as usize;
                // index + 1 + peer length
                if bytes.len() < index + 1 + peer_length{
                    return Err("invalid message")?;
                }
                let parsed_peer = Peer::from_bytes(&bytes[index+1..index+1+peer_length])?;
                peers.push(parsed_peer);
                index += peer_length + 1;
            }
            Ok(Self {
                sender,
                message_type,
                view: Some(peers)
            })
        }
        else {
            Ok(Self {
                sender,
                message_type,
                view: None
            })
        }
    }
}

impl Message for PeerSamplingMessage {
    /// Serializes the message to a vector of bytes
    fn as_bytes(&self) -> Vec<u8> {
        let mut buffer = vec![];
        // first byte: message type
        match self.message_type {
            MessageType::Request => buffer.push(MSG_TYPE_REQ),
            MessageType::Response => buffer.push(MSG_TYPE_RESP),
        }
        // sender
        buffer.push(self.sender.as_bytes().len() as u8);
        self.sender.as_bytes().iter().for_each(|byte| buffer.push(*byte));
        // view
        if let Some(peers) = &self.view {
            // view size in number of peers
            buffer.push(peers.len() as u8);
            // rest of bytes: peers
            peers.iter().map(|p| { p.as_bytes() }).for_each(|mut bytes| {
                // length of peer data in bytes
                buffer.push(bytes.len() as u8);
                // peer data
                buffer.append(&mut bytes);
            });
        } else {
            // empty set
            buffer.push(0);
        }
        buffer
    }
}