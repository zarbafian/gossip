use std::hash::{Hash, Hasher};
use serde::{Serialize, Deserialize};

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