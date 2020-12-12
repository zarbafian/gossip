use std::error::Error;
use rand::Rng;
use crate::message::Message;


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
}
impl Message for HeaderMessage {
    fn as_bytes(&self) -> Vec<u8> {
        let bytes = Vec::new();

        bytes
    }
    fn from_bytes(bytes: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        Ok(HeaderMessage {
            messages: vec![]
        })
    }
}

/// A generic message for sending any type of content
#[derive(Debug)]
pub struct ContentMessage {
    /// Message identifier
    id: String,
    /// Message content
    content: Vec<u8>
}

impl ContentMessage {
    /// Creates a new message
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
}

impl Message for ContentMessage {

    /// Serializes a message into bytes
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![crate::message::MESSAGE_PROTOCOL_DATA];
        let mut id = self.id.as_bytes().to_vec();
        bytes.push(id.len() as u8);
        bytes.append(&mut id);
        bytes.append(&mut self.content.to_vec());
        bytes
    }
    fn from_bytes(bytes: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        if bytes.len() < 3 { // message protocol + id length + id (>=1)
            Err("Invalid message with length less than 3 bytes")?
        }
        let id_length = bytes[1] as usize;
        if bytes.len() < 2 + id_length {
            Err("Invalid length for message identifier")?
        }
        let id = String::from_utf8(bytes[2..2+id_length].to_vec())?;

        Ok(ContentMessage {
            id,
            content: bytes[2+id_length..].to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ContentMessage;
    use crate::message::Message;

    #[test]
    fn serialize_deserialize() {
        let id = "toto-is-back";
        let content = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        // serialize
        let message = ContentMessage::new(id.to_owned(), content.clone());
        let bytes = message.as_bytes();

        // deserialize
        let parsed_message = ContentMessage::from_bytes(bytes).unwrap();
        assert_eq!(parsed_message.id(), id);
        assert_eq!(parsed_message.content(), &content)
    }

    #[test]
    fn serialize_deserialize_empty() {
        let id = "toto-is-back";
        let content = vec![];

        // serialize
        let message = ContentMessage::new(id.to_owned(), content.clone());
        let bytes = message.as_bytes();

        // deserialize
        let parsed_message = ContentMessage::from_bytes(bytes).unwrap();
        assert_eq!(parsed_message.id(), id);
        assert_eq!(parsed_message.content(), &content)
    }

    #[test]
    #[should_panic(expected = "Invalid message with length less than 3 bytes")]
    fn invalid_total_size() {
        ContentMessage::from_bytes(vec![crate::message::MESSAGE_TYPE_REQUEST | crate::message::MESSAGE_PROTOCOL_SAMPLING, 2]).unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid length for message identifier")]
    fn invalid_id_length() {
        ContentMessage::from_bytes(vec![crate::message::MESSAGE_TYPE_REQUEST | crate::message::MESSAGE_PROTOCOL_SAMPLING, 2, 33]).unwrap();
    }

    #[test]
    #[should_panic]
    fn message_id_length() {
        let mut id = String::new();
        for _ in 0..257 {
            id.push('1');
        }
        let content = vec![];

        let message = ContentMessage::new(id.to_owned(), content.clone());
    }
}