/// The peer sampling parameters
///
/// See: [Gossip-based Peer Sampling](https://infoscience.epfl.ch/record/109297/files/all.pdf)
#[derive(Clone)]
pub struct PeerSamplingConfig {
    push: bool,
    pull: bool,
    sampling_period: u64,
    sampling_deviation: u64,
    view_size: usize,
    healing_factor: usize,
    swapping_factor: usize,
}

impl PeerSamplingConfig {
    /// Create a new peer sampling configuration
    ///
    /// # Arguments
    ///
    /// * `push` - Does the node push its view to other peers
    /// * `pull` - When active, if the node will pull views from other peers; when passive, if it responds with its view to push requests
    /// * `sampling_period` - The interval between each cycle of push/pull
    /// * `view_size` - The number of peers in the view of the node
    /// * `healing_factor` - The number of removal at each cycle
    /// * `swapping_factor` - The number of peer swapped at each cycle
    pub fn new(push: bool, pull: bool, sampling_period: u64, view_size: usize, healing_factor: usize, swapping_factor: usize) -> Self {
        PeerSamplingConfig {
            push,
            pull,
            sampling_period,
            sampling_deviation: 0,
            view_size,
            healing_factor,
            swapping_factor,
        }
    }

    /// Creates a new configuration with the possibility to randomize the period; this is useful when testing locally in order to avoid network saturation
    /// # Arguments
    ///
    /// * `sampling_deviation` - The maximum value of the random value added to the period
    pub fn new_with_deviation(push: bool, pull: bool, sampling_period: u64, sampling_deviation: u64, view_size: usize, healing_factor: usize, swapping_factor: usize) -> Self {
        PeerSamplingConfig {
            push,
            pull,
            sampling_period,
            sampling_deviation,
            view_size,
            healing_factor,
            swapping_factor,
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
}

impl Default for PeerSamplingConfig {
    fn default() -> Self {
        PeerSamplingConfig {
            push: true,
            pull: true,
            sampling_period: 60000,
            sampling_deviation: 0,
            view_size: 30,
            healing_factor: 3,
            swapping_factor: 12
        }
    }
}

/// The gossip parameters
pub struct GossipConfig {
    push: bool,
    pull: bool,
    gossip_period: u64,
    gossip_deviation: u64,
    update_expiration: UpdateExpirationMode,
}

impl GossipConfig {
    /// Creates a new gossip configuration
    ///
    /// # Arguments
    ///
    /// * `push` - If the node push its content to other nodes
    /// * `pull` - When active, if the node will pull content from other peers; when passive, if the node responds with its content to push requests
    /// * `gossip_period` - Length of each gossip period
    /// * `update_expiration` - Strategy for update expiration, see [UpdateExpirationMode]
    pub fn new(push: bool, pull: bool, gossip_period: u64, update_expiration: UpdateExpirationMode) -> Self {
        GossipConfig {
            push,
            pull,
            gossip_period,
            gossip_deviation: 0,
            update_expiration,
        }
    }

    /// Creates a new configuration with the possibility to randomize the period; this is useful when testing locally in order to avoid network saturation
    /// # Arguments
    ///
    /// * `gossip_deviation` - The maximum value of the random value added to the period
    pub fn new_with_deviation(push: bool, pull: bool, gossip_period: u64, gossip_deviation: u64, update_expiration: UpdateExpirationMode) -> Self {
        GossipConfig {
            push,
            pull,
            gossip_period,
            gossip_deviation,
            update_expiration,
        }
    }
    pub fn is_push(&self) -> bool {
        self.push
    }
    pub fn is_pull(&self) -> bool {
        self.pull
    }
    pub fn gossip_period(&self) -> u64 {
        self.gossip_period
    }
    pub fn gossip_deviation(&self) -> u64 {
        self.gossip_deviation
    }
    pub fn update_expiration(&self) -> &UpdateExpirationMode {
        &self.update_expiration
    }
}

impl Default for GossipConfig {
    fn default() -> Self {
        GossipConfig {
            push: true,
            pull: true,
            gossip_period: 1000,
            gossip_deviation: 0,
            update_expiration: UpdateExpirationMode::None
        }
    }
}

/// Strategy for update expiration
#[derive(Debug, Clone)]
pub enum UpdateExpirationMode {
    /// Updates never expire
    None,
    /// Updates expire after the specified duration (milliseconds)
    DurationMillis(u128),
    /// Updates expire after being pushed the specified number of times
    PushCount(u64),
    /// Only the specified count of the most recent updates
    MostRecent(usize, f64),
}

pub enum UpdateExpirationValue {
    None,
    DurationMillis(std::time::Instant, u128),
    PushCount(u64),
    MostRecent(std::time::Instant),
}
impl UpdateExpirationValue {
    pub fn new(expiration_mode: UpdateExpirationMode) -> Self {
        match expiration_mode {
            UpdateExpirationMode::None => UpdateExpirationValue::None,
            UpdateExpirationMode::PushCount(count) => UpdateExpirationValue::PushCount(count),
            UpdateExpirationMode::DurationMillis(ms) => UpdateExpirationValue::DurationMillis(std::time::Instant::now(), ms),
            UpdateExpirationMode::MostRecent(_, _) => UpdateExpirationValue::MostRecent(std::time::Instant::now()),
        }
    }

    pub fn increase_push_count(&mut self) {
        match self {
            // increase push count
            UpdateExpirationValue::PushCount(ref mut count) => {
                if *count > 0 {
                    *count -= 1
                }
            },
            _ => (),
        }
    }

    pub fn has_expired(&self) -> bool {
        match self {
            UpdateExpirationValue::None => false,
            UpdateExpirationValue::PushCount(count) => *count == 0,
            UpdateExpirationValue::DurationMillis(start, ttl) => start.elapsed().as_millis() >= *ttl,
            UpdateExpirationValue::MostRecent(_) => false,
        }
    }
}