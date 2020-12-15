mod common;

use gossip::{GossipService, GossipConfig, PeerSamplingConfig, Peer, UpdateExpirationMode};
use crate::common::TextMessageListener;

#[test]
#[should_panic (expected = "Message already active")]
fn submit_active() {
    common::configure_logging(log::LevelFilter::Debug).unwrap();

    let sampling_period = 2000;
    let sampling_deviation = 1;
    let gossip_period = 1000;
    let gossip_deviation = 1;
    let expiration_mode = UpdateExpirationMode::DurationMillis(3000);

    let address_1 = "127.0.0.1:9000";
    let mut service_1 = GossipService::new(
        address_1.parse().unwrap(),
        PeerSamplingConfig::new_with_deviation(true, true, sampling_period, sampling_deviation, 10, 1, 4),
        GossipConfig::new_with_deviation(true, true, gossip_period, gossip_deviation, expiration_mode.clone()),
        None
    );
    service_1.start(
        Box::new( || None),
        Box::new(TextMessageListener::new(address_1.to_owned()))
    ).unwrap();

    // JSON message
    let message_content_1 = "{{ \"id\": \"toto\", \"name\": \"John Doe\" }}";

    let result = service_1.submit(message_content_1.as_bytes().to_vec());
    assert!(result.is_ok());
    service_1.submit(message_content_1.as_bytes().to_vec()).unwrap();
}
