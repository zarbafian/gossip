mod common;

use gossip::{GossipService, GossipConfig, PeerSamplingConfig};
use log::LevelFilter;

#[test]
fn start_gossip() {
/*
    common::configure_logging(log::LevelFilter::Debug).unwrap();

    let address_1 = "127.0.0.1:9000";
    let mut service_1 = GossipService::new(
        PeerSamplingConfig::new(address_1.parse().unwrap(), true, true, 5, 0, 10, 1, 4, None),
        GossipConfig::new(address_1.parse().unwrap(), 1000)
    );
    service_1.start(Box::new( || None));

    let address_2 = "127.0.0.1:9001";
    let mut service_2 = GossipService::new(
        PeerSamplingConfig::new(address_2.parse().unwrap(), true, true, 5, 0, 10, 1, 4, None),
        GossipConfig::new(address_2.parse().unwrap(), 1000)
    );
    service_2.start(Box::new(move || Some(vec![gbps::Peer::new(address_1.to_owned())])));


    std::thread::sleep(std::time::Duration::from_secs(5));

    service_1.shutdown();
    service_2.shutdown();
 */
}