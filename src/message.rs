pub mod gossip;
pub mod sampling;

use std::error::Error;
use crate::message::sampling::PeerSamplingMessage;
use crate::message::gossip::{ContentMessage, HeaderMessage};

// Protocol is the first four bits, message type is the next four bits
pub const MASK_PROTOCOL: u8             = 0xF0; // 0b11110000
pub const MESSAGE_PROTOCOL_DATA: u8     = 0x10; // 0b00010000
pub const MESSAGE_PROTOCOL_SAMPLING: u8 = 0x20; // 0b00100000
pub const MESSAGE_PROTOCOL_HEADER: u8   = 0x40; // 0b01000000
pub const MESSAGE_PROTOCOL_NOOP: u8     = 0x40; // 0b01000000
pub const MASK_MESSAGE_TYPE: u8         = 0x0F; // 0b00001111
pub const MESSAGE_TYPE_REQUEST: u8      = 0x01; // 0b00000001
pub const MESSAGE_TYPE_RESPONSE: u8     = 0x02; // 0b00000010

pub trait Message {
    fn as_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: Vec<u8>) -> Result<Self, Box<dyn Error>> where Self: Sized;
}

pub struct PeerSamplingMessageParser;
impl PeerSamplingMessageParser {
    pub fn parse(&self, bytes: Vec<u8>) -> Result<PeerSamplingMessage, Box<dyn Error>> {
        PeerSamplingMessage::from_bytes(bytes)
    }
}
pub struct HeaderMessageParser;
impl HeaderMessageParser {
    pub fn parse(&self, bytes: Vec<u8>) -> Result<HeaderMessage, Box<dyn Error>> {
        HeaderMessage::from_bytes(bytes)
    }
}
pub struct ContentMessageParser;
impl ContentMessageParser {
    pub fn parse(&self, bytes: Vec<u8>) -> Result<ContentMessage, Box<dyn Error>> {
        ContentMessage::from_bytes(bytes)
    }
}

pub struct NoopMessage;
impl Message for NoopMessage {
    fn as_bytes(&self) -> Vec<u8> {
        vec![ MESSAGE_PROTOCOL_NOOP ]
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        Ok(NoopMessage)
    }
}