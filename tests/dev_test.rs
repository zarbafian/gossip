mod common;

use gossip::{GossipService, GossipConfig, PeerSamplingConfig, Peer};
use log::LevelFilter;

struct TextMessageListener {id: String}
impl TextMessageListener {
    fn on_message(&self, bytes: Vec<u8>) {
        log::info!("--------------------------");
        log::info!("[{}] Received message:", self.id);
        log::info!("{}", String::from_utf8(bytes).unwrap());
        log::info!("--------------------------");
    }
}

#[test]
fn start_gossip() {
    common::configure_logging(log::LevelFilter::Debug).unwrap();

    let sampling_period = 2;
    let sampling_deviation = 2;
    let gossip_period = 2000;

    let address_1 = "127.0.0.1:9000";
    let listener_1 = TextMessageListener { id: address_1.to_owned() };
    let mut service_1 = GossipService::new(
        address_1.parse().unwrap(),
        PeerSamplingConfig::new(true, true, sampling_period, sampling_deviation, 10, 1, 4, None),
        GossipConfig::new(address_1.parse().unwrap(), gossip_period)
    );
    service_1.start(Box::new( || None));

    let address_2 = "127.0.0.1:9001";
    let listener_2 = TextMessageListener { id: address_2.to_owned() };
    let mut service_2 = GossipService::new(
        address_2.parse().unwrap(),
        PeerSamplingConfig::new(true, true, sampling_period, sampling_deviation, 10, 1, 4, None),
        GossipConfig::new(address_2.parse().unwrap(), gossip_period)
    );
    service_2.start(Box::new(move || Some(vec![Peer::new(address_1.to_owned())])));

    // wait for peer discovery
    std::thread::sleep(std::time::Duration::from_secs(3 * (sampling_deviation + sampling_deviation)));

    // JSON message
    let message_id_1 = "abcd-efgh";
    let message_content_1 = "{{ \"id\": \"toto\", \"name\": \"John Doe\" }}";
    service_1.submit(message_id_1.to_owned(), message_content_1.as_bytes().to_vec());
    std::thread::sleep(std::time::Duration::from_secs(1));

    // binary message
    let message_id_2 = "ijkl-mnop";
    let message_content_2 = "Just some simple text for Toto!\nBut why?";
    service_2.submit(message_id_2.to_owned(), message_content_2.as_bytes().to_vec());

    std::thread::sleep(std::time::Duration::from_secs(1));

    // wait for gossip
    std::thread::sleep(std::time::Duration::from_secs(5));

    service_1.shutdown();
    service_2.shutdown();
}