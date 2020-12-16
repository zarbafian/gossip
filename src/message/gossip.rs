use serde::{Serialize, Deserialize};
use crate::message::{Message, MESSAGE_PROTOCOL_HEADER_MESSAGE, MESSAGE_PROTOCOL_CONTENT_MESSAGE, MessageType};
use std::collections::HashMap;

/// A message containing the digests of all the active updates of the node.
/// It is used to advertise the updates present at each node.
#[derive(Debug, Serialize, Deserialize)]
pub struct HeaderMessage {
    sender: String,
    message_type: MessageType,
    messages: Vec<String>,
}
impl HeaderMessage {
    pub fn new_request(sender: String) -> Self {
        Self::new(sender, MessageType::Request)
    }
    pub fn new_response(sender: String) -> Self {
        Self::new(sender, MessageType::Response)
    }
    fn new(sender: String, message_type: MessageType) -> Self {
        HeaderMessage {
            sender,
            message_type,
            messages: Vec::new()
        }
    }
    pub fn push(&mut self, message_digest: String) {
        self.messages.push(message_digest);
    }
    pub fn sender(&self) -> &str {
        &self.sender
    }
    pub fn message_type(&self) -> &MessageType {
        &self.message_type
    }
    pub fn messages(&self) -> &Vec<String> {
        &self.messages
    }
}
impl Message for HeaderMessage {
    fn protocol(&self) -> u8 {
        MESSAGE_PROTOCOL_HEADER_MESSAGE
    }
}

/// A message containing the updates.
/// A [MessageType::Request] is used to query for updates, a [MessageType::Response] is used to send updates.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContentMessage {
    sender: String,
    message_type: MessageType,
    content: HashMap<String, Vec<u8>>,
}
impl ContentMessage {
    pub fn new_request(sender: String, content: HashMap<String, Vec<u8>>) -> Self {
        Self::new(sender, MessageType::Request, content)
    }
    pub fn new_response(sender: String, content: HashMap<String, Vec<u8>>) -> Self {
        Self::new(sender, MessageType::Response, content)
    }
    fn new(sender: String, message_type: MessageType, content: HashMap<String, Vec<u8>>) -> Self {
        ContentMessage {
            sender,
            message_type,
            content,
        }
    }
    pub fn sender(&self) -> &str {
        &self.sender
    }
    pub fn message_type(&self) -> &MessageType {
        &self.message_type
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }
    /// Returns the content of the message. Moves the message to avoid copying its content.
    pub fn content(self) -> HashMap<String, Vec<u8>> {
        self.content
    }
}
impl Message for ContentMessage {
    fn protocol(&self) -> u8 {
        MESSAGE_PROTOCOL_CONTENT_MESSAGE
    }
}

/// A generic message for sending data as binary content
#[derive(Debug, Serialize, Deserialize)]
pub struct Update {
    /// Message content
    content: Vec<u8>,
    /// Content digest
    digest: String,
}

impl Update {
    /// Creates a new message with generic content
    ///
    /// # Arguments
    ///
    /// * `content` - Message content
    /// * `digest` - Content digest
    pub fn new(content: Vec<u8>) -> Self {
        let digest = blake3::hash(&content).to_hex().to_string();
        Update {
            content,
            digest,
        }
    }

    pub fn content(&self) -> &Vec<u8> {
        &self.content
    }

    pub fn digest(&self) -> &String {
        &self.digest
    }
}

/// Trait for receiving updates from the gossip protocol.
///
/// See: [Update]
pub trait UpdateHandler {
    /// Method called every time a new update is available for the application layer
    ///
    /// # Arguments
    ///
    /// * `update` - The update that has been received
    fn on_update(&self, update: Update);
}
