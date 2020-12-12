use std::hash::{Hash, Hasher};
use std::error::Error;
use serde::{Serialize, Deserialize};
// TODO: Refactor
// Byte separator between the peer address and the peer age
const SEPARATOR: u8 = 0x2C; // b','

/// Information about a peer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peer {
    /// Socket address of the peer
    address: String,
    /// Age of the peer
    age: u16,
}

impl Peer {
    /// Creates a new peer with the specified address and age 0
    ///
    /// # Arguments
    ///
    /// * `address` - Network address of peer
    pub fn new(address: String) -> Peer {
        Peer {address, age: 0}
    }

    /// Increments the age of peer by one
    pub fn increment_age(&mut self) {
        if self.age < u16::max_value() {
            self.age += 1;
        }
    }

    /// Returns the age of peer
    pub fn age(&self) -> u16 {
        self.age
    }

    /// Returns the address of peer
    pub fn address(&self) -> &str { &self.address }

    /// Serializes peer into an array of bytes.
    /// Starts with the address of the peer first followed by the age of the peer
    /// address and age are separated by a [SEPARATOR] byte.
    pub fn as_bytes(&self) -> Vec<u8> {
        // peer address
        let mut v = self.address.as_bytes().to_vec();
        // separator
        v.push(SEPARATOR);
        // peer age: first byte
        v.push((self.age >> 8) as u8);
        // peer age: second byte
        v.push((self.age & 0x00FF) as u8);
        v
    }

    /// Deserializes a peer from an array of bytes
    ///
    /// # Arguments
    ///
    /// * `bytes` - A peer serialized as bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Peer, Box<dyn Error>> {
        // retrieve index of separator
        let separator_index = bytes.iter().enumerate()
            .find(|(_, b)| { **b == SEPARATOR})
            .map(|(i, _)| {i});
        if let Some(index) = separator_index {
            // check that there are exactly two bytes for the age after separator
            if bytes.len() != index + 3 {
                Err("invalid age")?
            }
            // retrieve address
            let address = String::from_utf8(bytes[..index].to_vec())?;
            // build age
            let age = ((bytes[index+1] as u16) << 8 ) + (bytes[index+2] as u16);
            Ok(Peer{
                address,
                age,
            })
        }
        else {
            Err("peer separator not found")?
        }
    }
}
impl Eq for Peer {}
impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}
impl Hash for Peer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state)
    }
}