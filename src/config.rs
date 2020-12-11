use std::net::SocketAddr;

pub struct GossipConfig {
    address: SocketAddr,
    gossip_interval: u64,
}

impl GossipConfig {
    pub fn new(address: SocketAddr, gossip_interval: u64) -> Self {
        GossipConfig {
            address,
            gossip_interval
        }
    }
    pub fn address(&self) -> &SocketAddr {
        &self.address
    }
    pub fn gossip_interval(&self) -> u64 {
        self.gossip_interval
    }
}