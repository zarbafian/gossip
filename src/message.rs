pub mod gossip;
pub mod sampling;

use std::error::Error;
use serde::{Deserialize, Serialize};

// Protocol is the first four bits
pub const MASK_MESSAGE_PROTOCOL: u8             = 0xF0; // 0b11110000
pub const MESSAGE_PROTOCOL_SAMPLING_MESSAGE: u8 = 0x10; // 0b00010000
pub const MESSAGE_PROTOCOL_HEADER_MESSAGE: u8   = 0x20; // 0b00100000
pub const MESSAGE_PROTOCOL_CONTENT_MESSAGE: u8  = 0x40; // 0b01000000
pub const MESSAGE_PROTOCOL_NOOP_MESSAGE: u8     = 0x80; // 0b10000000

/// The message type. [MessageType::Request] is used to advertise the node data or request advertised data;
/// [MessageType::Response] is used to advertise back in response to a request, or provide the requested data.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Request = 1,
    Response = 2,
}

/// Message trait with generic implementation for serialization and deserialization
pub trait Message {

    /// The message protocol, used for serialization/deserialization
    fn protocol(&self) -> u8;

    /// Serializes message for sending over the wire
    fn as_bytes(&self) -> Result<Vec<u8>, Box<dyn Error>>
    where Self: Serialize
    {
        match serde_cbor::to_vec(&self) {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(e)?,
        }
    }

    /// Deserializes a message
    fn from_bytes<'a>(bytes: &'a [u8]) -> Result<Self, Box<dyn Error>>
    where Self: Sized + Deserialize<'a>
    {
        match serde_cbor::from_slice::<Self>(bytes) {
            Ok(m) => Ok(m),
            Err(e) => Err(e)?
        }
    }
}

/// An empty no-op message used to stop listening TCP connections
#[derive(Debug, Serialize, Deserialize)]
pub struct NoopMessage;
impl Message for NoopMessage {
    fn protocol(&self) -> u8 {
        MESSAGE_PROTOCOL_NOOP_MESSAGE
    }
}
