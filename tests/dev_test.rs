mod common;

use gossip::{GossipService, GossipConfig, PeerSamplingConfig, Peer, UpdateHandler, Update, UpdateExpirationMode};
use log::LevelFilter;
use crate::common::TextMessageListener;

#[test]
fn start_gossip() {
    common::configure_logging(log::LevelFilter::Debug).unwrap();

    let sampling_period = 3000;
    let sampling_deviation = 3000;
    let gossip_period = 2000;
    let gossip_deviation = 2000;
    let expiration_mode = UpdateExpirationMode::DurationMillis(3000);

    let address_1 = "127.0.0.1:9000";
    let mut service_1 = GossipService::new(
        address_1.parse().unwrap(),
        PeerSamplingConfig::new_with_deviation(true, true, sampling_period, sampling_deviation, 10, 1, 4),
        GossipConfig::new_with_deviation(true, true, gossip_period, gossip_deviation, expiration_mode.clone())
    );
    service_1.start(
        Box::new( || None),
        Box::new(TextMessageListener::new(address_1.to_owned()))
    );

    let address_2 = "127.0.0.1:9001";
    let mut service_2 = GossipService::new(
        address_2.parse().unwrap(),
        PeerSamplingConfig::new_with_deviation(true, true, sampling_period, sampling_deviation, 10, 1, 4),
        GossipConfig::new_with_deviation(true, true, gossip_period, gossip_deviation, expiration_mode)
    );
    service_2.start(
        Box::new(move || Some(vec![Peer::new(address_1.to_owned())])),
        Box::new(TextMessageListener::new(address_2.to_owned()))
    );

    // wait for peer discovery
    std::thread::sleep(std::time::Duration::from_millis(3 * (sampling_deviation + sampling_deviation)));

    // JSON message
    let message_content_1 = "{{ \"id\": \"toto\", \"name\": \"John Doe\" }}";
    service_1.submit(message_content_1.as_bytes().to_vec());
    std::thread::sleep(std::time::Duration::from_secs(1));

    // binary message
    let message_content_2 = "Just some simple text for Toto!\nBut why?";
    service_2.submit(message_content_2.as_bytes().to_vec());

    std::thread::sleep(std::time::Duration::from_secs(1));

    // wait for expiration
    std::thread::sleep(std::time::Duration::from_secs(10));

    service_1.shutdown();
    service_2.shutdown();
}