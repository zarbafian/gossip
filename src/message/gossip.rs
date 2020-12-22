use serde::{Serialize, Deserialize};
use crate::message::{Message, MESSAGE_PROTOCOL_HEADER_MESSAGE, MESSAGE_PROTOCOL_CONTENT_MESSAGE, MessageType};
use std::collections::HashMap;

/// A message containing the digests of all the active updates on a node.
/// It is used to advertise the updates present at each node.
#[derive(Debug, Serialize, Deserialize)]
pub struct HeaderMessage {
    sender: String,
    message_type: MessageType,
    headers: Vec<String>,
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
            headers: Vec::new()
        }
    }
    pub fn set_headers(&mut self, headers: Vec<String>) {
        self.headers = headers
    }
    pub fn sender(&self) -> &str {
        &self.sender
    }
    pub fn message_type(&self) -> &MessageType {
        &self.message_type
    }
    pub fn headers(&self) -> &Vec<String> {
        &self.headers
    }
}
impl Message for HeaderMessage {
    fn protocol(&self) -> u8 {
        MESSAGE_PROTOCOL_HEADER_MESSAGE
    }
}

/// A message that is either a request for updates ([MessageType::Request]) or a response
/// containing requested updates ([MessageType::Response]).
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
