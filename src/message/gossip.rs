use std::error::Error;
use rand::Rng;
use crate::message::Message;

#[derive(Debug)]
pub struct GossipMessage {
    content: String
}

impl GossipMessage {
    pub fn new() -> Self {
        GossipMessage{
            content: format!("{:x}", rand::thread_rng().gen_range(1, 65536))
        }
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        Ok(GossipMessage{
            content: String::from_utf8(bytes).unwrap()
        })
    }
}

impl Message for GossipMessage {
    fn as_bytes(&self) -> Vec<u8> {
        unimplemented!()
    }
}