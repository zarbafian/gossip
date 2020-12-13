mod common;

use gossip::{GossipService, GossipConfig, PeerSamplingConfig, Peer, UpdateHandler, Update};
use log::LevelFilter;

struct TextMessageListener {id: String}
impl UpdateHandler for TextMessageListener {
    fn on_update(&self, update: Update) {
        log::info!("--------------------------");
        log::info!("[{}] Received message:", self.id);
        log::info!("{}", String::from_utf8(update.content().to_vec()).unwrap());
        log::info!("--------------------------");
    }
}

#[test]
fn start_gossip() {
    common::configure_logging(log::LevelFilter::Info).unwrap();

    let sampling_period = 2;
    let sampling_deviation = 2;
    let gossip_period = 2000;
    let gossip_deviation = 2000;

    let address_1 = "127.0.0.1:9000";
    let listener_1 = TextMessageListener { id: address_1.to_owned() };
    let mut service_1 = GossipService::new(
        address_1.parse().unwrap(),
        PeerSamplingConfig::new(true, true, sampling_period, sampling_deviation, 10, 1, 4),
        GossipConfig::new(true, true, address_1.parse().unwrap(), gossip_period, gossip_deviation),
        None
    );
    service_1.start(
        Box::new( || None),
        Box::new(TextMessageListener {id : "John".to_owned()})
    );

    let address_2 = "127.0.0.1:9001";
    let listener_2 = TextMessageListener { id: address_2.to_owned() };
    let mut service_2 = GossipService::new(
        address_2.parse().unwrap(),
        PeerSamplingConfig::new(true, true, sampling_period, sampling_deviation, 10, 1, 4),
        GossipConfig::new(true, true,address_2.parse().unwrap(), gossip_period, gossip_deviation),
        None
    );
    service_2.start(
        Box::new(move || Some(vec![Peer::new(address_1.to_owned())])),
        Box::new(TextMessageListener {id : "Billy".to_owned()})
    );

    // wait for peer discovery
    std::thread::sleep(std::time::Duration::from_secs(3 * (sampling_deviation + sampling_deviation)));

    // JSON message
    let message_content_1 = "{{ \"id\": \"toto\", \"name\": \"John Doe\" }}";
    service_1.submit(message_content_1.as_bytes().to_vec());
    std::thread::sleep(std::time::Duration::from_secs(1));

    // binary message
    let message_content_2 = "Just some simple text for Toto!\nBut why?";
    service_2.submit(message_content_2.as_bytes().to_vec());

    std::thread::sleep(std::time::Duration::from_secs(1));

    // wait for gossip
    std::thread::sleep(std::time::Duration::from_secs(5));

    service_1.shutdown();
    service_2.shutdown();
}