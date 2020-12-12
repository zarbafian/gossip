use std::fmt::Debug;
use serde::{Serialize, Deserialize};
use crate::peer::Peer;
use crate::message::{self, Message, MESSAGE_PROTOCOL_SAMPLING_MESSAGE};

/// A peer sampling protocol message
#[derive(Debug, Serialize, Deserialize)]
pub struct PeerSamplingMessage {
    /// Address of the sender
    sender: String,
    /// Type of the message
    message_type: message::MessageType,
    /// The view of the sender
    view: Option<Vec<Peer>>,
}

impl PeerSamplingMessage {
    /// Creates a new message of type [MessageType::Request] containing a view
    pub fn new_request(sender: String, view: Option<Vec<Peer>>) -> Self {
        Self::new(sender, message::MessageType::Request, view)
    }

    /// Creates a new message of type [MessageType::Response] containing a view
    pub fn new_response(sender: String, view: Option<Vec<Peer>>) -> Self {
        Self::new(sender, message::MessageType::Response, view)
    }

    fn new(sender: String, message_type: message::MessageType, view: Option<Vec<Peer>>) -> Self {
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
    pub fn message_type(&self) -> &message::MessageType {
        &self.message_type
    }

    /// Returns the view contained in the message
    pub fn view(&self) -> &Option<Vec<Peer>> {
        &self.view
    }
}

impl Message for PeerSamplingMessage {
    fn protocol(&self) -> u8 {
        MESSAGE_PROTOCOL_SAMPLING_MESSAGE
    }
}
