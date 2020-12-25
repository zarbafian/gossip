mod common;

#[test]
fn all_updates_received() {
    use gossip::{GossipConfig, PeerSamplingConfig, Peer, GossipService, Update, UpdateExpirationMode};
    use common::NoopUpdateHandler;

    common::configure_logging(log::LevelFilter::Info).unwrap();

    let mut messages = Vec::new();

    // algorithm parameters
    let gossip_period = 600;

    let sampling_period = 1000;
    let push = true;
    let pull = true;
    let c = 30;
    let h = 3;
    let s = 12;

    let push_count = 5;
    let update_expiration = UpdateExpirationMode::PushCount(push_count);

    // create first peer with no contact peer
    let initial_peer = "127.0.0.1:9000";

    // create and initiate the peer sampling service
    let mut service_1 = GossipService::new(
        initial_peer.parse().unwrap(),
        PeerSamplingConfig::new(push, pull, sampling_period, c, h, s),
        GossipConfig::new(push, pull, gossip_period, update_expiration.clone())
    );
    service_1.start(
        Box::new(move|| { None }),
        Box::new(NoopUpdateHandler)
    );

    // create second peer
    let init_handler = Box::new(move|| { Some(vec![Peer::new(initial_peer.to_owned())]) });

    // create and initiate the gossip service
    let mut service_2 = GossipService::new(
        "127.0.0.1:9001".parse().unwrap(),
        PeerSamplingConfig::new(push, pull, sampling_period, c, h, s),
        GossipConfig::new(push, pull, gossip_period, update_expiration.clone())
    );
    service_2.start(
        init_handler,
        Box::new(NoopUpdateHandler)
    );

    // initializing peer sampling
    std::thread::sleep(std::time::Duration::from_millis(sampling_period * 2));

    let message_count = 10;

    for i in 0..message_count {
        let message = format!("MSGID {}", i).as_bytes().to_vec();
        let update = Update::new(message.clone());
        service_2.submit(update.content().to_vec());
        messages.push(message);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // wait for messages expiration
    std::thread::sleep(std::time::Duration::from_millis(gossip_period * (push_count + 1)));

    for message in messages {
        assert!(service_2.is_expired(message));
    }

    service_1.shutdown();
    service_2.shutdown();
}
