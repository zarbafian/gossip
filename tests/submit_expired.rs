mod common;

use gossip::{GossipService, GossipConfig, PeerSamplingConfig, Peer, UpdateExpirationMode};
use crate::common::TextMessageHandler;

#[test]
fn submit_expired() {
    common::configure_logging(log::LevelFilter::Debug).unwrap();

    let sampling_period = 2000;
    let sampling_deviation = 1;
    let gossip_period = 1000;
    let gossip_deviation = 1;
    let expiration_mode = UpdateExpirationMode::DurationMillis(1500);

    let address_1 = "127.0.0.1:9000";
    let mut service_1 = GossipService::new(
        address_1.parse().unwrap(),
        PeerSamplingConfig::new_with_deviation(true, true, sampling_period, sampling_deviation, 10, 1, 4),
        GossipConfig::new_with_deviation(true, true, gossip_period, gossip_deviation, expiration_mode.clone())
    );
    service_1.start(
        Box::new( || None),
        Box::new(TextMessageHandler::new(address_1.to_owned()))
    ).unwrap();

    let address_2 = "127.0.0.1:9001";
    let mut service_2 = GossipService::new(
        address_2.parse().unwrap(),
        PeerSamplingConfig::new_with_deviation(true, true, sampling_period, sampling_deviation, 10, 1, 4),
        GossipConfig::new_with_deviation(true, true, gossip_period, gossip_deviation, expiration_mode)
    );
    service_2.start(
        Box::new(move || Some(vec![Peer::new(address_1.to_owned())])),
        Box::new(TextMessageHandler::new(address_2.to_owned()))
    ).unwrap();

    // wait for peer discovery
    std::thread::sleep(std::time::Duration::from_millis(2 * (sampling_deviation + sampling_deviation)));

    // JSON message
    let message_content_1 = "{{ \"id\": \"toto\", \"name\": \"John Doe\" }}";
    service_1.submit(message_content_1.as_bytes().to_vec()).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));

    // wait for expiration
    std::thread::sleep(std::time::Duration::from_secs(10));

    assert!(service_1.is_expired(message_content_1.as_bytes().to_vec()));
}