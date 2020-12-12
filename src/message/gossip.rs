use serde::{Serialize, Deserialize};
use crate::message::{Message, MESSAGE_PROTOCOL_HEADER_MESSAGE, MESSAGE_PROTOCOL_CONTENT_MESSAGE};

#[derive(Debug, Serialize, Deserialize)]
pub struct HeaderMessage {
    messages: Vec<(String, String)>,
}
impl HeaderMessage {
    pub fn new() -> Self {
        HeaderMessage {
            messages: Vec::new()
        }
    }
    pub fn push(&mut self, message_id: String, message_digest: String) {
        self.messages.push((message_id, message_digest));
    }
    pub fn messages(&self) -> &Vec<(String, String)> {
        &self.messages
    }
}
impl Message for HeaderMessage {
    fn protocol(&self) -> u8 {
        MESSAGE_PROTOCOL_HEADER_MESSAGE
    }
}

/// A generic message for sending data as binary content
#[derive(Debug, Serialize, Deserialize)]
pub struct ContentMessage {
    /// Message identifier
    id: String,
    /// Message content
    content: Vec<u8>
}

impl ContentMessage {
    /// Creates a new message with a generic content
    ///
    /// # Arguments
    ///
    /// `id` - Message identifier (must be less than 256 bytes)
    /// `content` - Message content
    pub fn new(id: String, content: Vec<u8>) -> Self {
        assert!(id.as_bytes().len() < u8::max_value() as usize);
        ContentMessage {
            id,
            content,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn content(&self) -> &Vec<u8> {
        &self.content
    }

    pub fn digest(&self) -> String {
        // TODO
        // TODO
        // TODO
        let mut hexa = String::new();
        self.content.iter().for_each(|byte| hexa.push_str(&format!("{:x}", byte)));
        hexa
    }
}

impl Message for ContentMessage {
    fn protocol(&self) -> u8 {
        MESSAGE_PROTOCOL_CONTENT_MESSAGE
    }
}
