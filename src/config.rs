use std::net::SocketAddr;
use crate::monitor::MonitoringConfig;

/// The peer sampling parameters
///
/// See: https://infoscience.epfl.ch/record/109297/files/all.pdf
#[derive(Clone)]
pub struct PeerSamplingConfig {
    /// Does the node push its view to other peers
    push: bool,
    /// When active, if the node will pull views from other peers
    /// When passive, if it responds with its view to pull from other peers
    pull: bool,
    /// The interval between each cycle of push/pull
    sampling_period: u64,
    /// Maximum value of random deviation added to the sampling interval.
    /// Intended for local testing.
    sampling_deviation: u64,
    /// The number of peers in the node's view
    view_size: usize,
    /// The number of removal at each cycle
    healing_factor: usize,
    /// The number of peer swapped at each cycle
    swapping_factor: usize,
    /// Monitoring configuration
    monitoring: MonitoringConfig,
}

impl PeerSamplingConfig {
    /// Returns a configuration with specified parameters
    pub fn new(push: bool, pull: bool, sampling_period: u64, sampling_deviation: u64, view_size: usize, healing_factor: usize, swapping_factor: usize, monitoring_config: Option<MonitoringConfig>) -> Self {
        let monitoring = match monitoring_config {
            Some(config) => config,
            None => MonitoringConfig::default(),
        };
        PeerSamplingConfig {
            push,
            pull,
            sampling_period,
            sampling_deviation,
            view_size,
            healing_factor,
            swapping_factor,
            monitoring
        }
    }

    pub fn sampling_period(&self) -> u64 {
        self.sampling_period
    }

    pub fn sampling_deviation(&self) -> u64 {
        self.sampling_deviation
    }

    pub fn healing_factor(&self) -> usize {
        self.healing_factor
    }

    pub fn swapping_factor(&self) -> usize {
        self.swapping_factor
    }

    pub fn view_size(&self) -> usize {
        self.view_size
    }

    pub fn is_pull(&self) -> bool {
        self.pull
    }

    pub fn is_push(&self) -> bool {
        self.push
    }

    pub fn monitoring(&self) -> &MonitoringConfig {
        &self.monitoring
    }
}

/// The gossip parameters
pub struct GossipConfig {
    address: SocketAddr,
    gossip_interval: u64,
}

impl GossipConfig {
    pub fn new(address: SocketAddr, gossip_interval: u64) -> Self {
        GossipConfig {
            address,
            gossip_interval,
        }
    }
    pub fn address(&self) -> &SocketAddr {
        &self.address
    }
    pub fn gossip_interval(&self) -> u64 {
        self.gossip_interval
    }
}