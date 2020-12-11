mod common;

use gossip::{GossipService, GossipConfig};
use log::LevelFilter;

#[test]
fn start_gossip() {

    common::configure_logging(log::LevelFilter::Debug).unwrap();

    let mut service_1 = GossipService::new(
        GossipConfig::new("127.0.0.1:9000".parse().unwrap(), 1000)
    );
    service_1.start();

    let mut service_2 = GossipService::new(
        GossipConfig::new("127.0.0.1:9001".parse().unwrap(), 1000)
    );
    service_2.start();


    std::thread::sleep(std::time::Duration::from_secs(5));

    service_1.shutdown();
    service_2.shutdown();
}