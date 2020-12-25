mod common;

#[test]
fn all_updates_received() {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use rand::Rng;
    use gossip::{GossipConfig, PeerSamplingConfig, Peer, GossipService, Update, UpdateExpirationMode};
    use common::MapUpdatingHandler;

    common::configure_logging(log::LevelFilter::Info).unwrap();

    // messages received by each peer
    let mut peer_messages = Arc::new(Mutex::new(HashMap::new()));

    // algorithm parameters
    let gossip_period = 400;

    let sampling_period = 1000;
    let push = true;
    let pull = true;
    let c = 30;
    let h = 3;
    let s = 12;

    let update_expiration = UpdateExpirationMode::None;

    let peer_count = 10;
    let mut instances = vec![];

    // create first peer with no contact peer
    let init_peer = "127.0.0.1:9000";
    // no contact peer for first node
    let no_peer_handler = Box::new(move|| { None });

    // create and initiate the peer sampling service
    let mut service = GossipService::new(
        init_peer.parse().unwrap(),
        PeerSamplingConfig::new(push, pull, sampling_period, c, h, s),
        GossipConfig::new(push, pull, gossip_period, update_expiration.clone())
    );
    service.start(no_peer_handler, Box::new(MapUpdatingHandler::new(init_peer.to_owned(), Arc::clone(&peer_messages))));
    instances.push(service);

    let mut port = 9001;
    for i in 1..peer_count {
        // peer socket address
        let address = format!("127.0.0.1:{}", port);
        // closure for retrieving the address of the first contact peer
        let init_handler = Box::new(move|| { Some(vec![Peer::new(init_peer.to_owned())]) });

        // create and initiate the gossip service
        let mut ipv4_service = GossipService::new(
            address.parse().unwrap(),
            PeerSamplingConfig::new(push, pull, sampling_period, c, h, s),
            GossipConfig::new(push, pull, gossip_period, update_expiration.clone())
        );
        ipv4_service.start(init_handler, Box::new(MapUpdatingHandler::new(address.clone(), Arc::clone(&peer_messages))));
        instances.push(ipv4_service);

        port += 1;
    }

    // wait for peer sampling initialization
    std::thread::sleep(std::time::Duration::from_millis(sampling_period * 5));

    let message_count = 10;
    let mut all_messages = Vec::with_capacity(message_count);

    for i in 0..message_count {
        let message = format!("MSGID {}", i).as_bytes().to_vec();
        let update = Update::new(message.clone());
        let my_digest = update.digest().clone();
        all_messages.push(update);
        let selected_peer = rand::thread_rng().gen_range(0, instances.len());
        instances[selected_peer].submit(message);
        {
            peer_messages.lock().unwrap().entry(instances[selected_peer].address().to_string()).or_insert(Vec::new()).push(my_digest);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    // wait for broadcast
    std::thread::sleep(std::time::Duration::from_millis(gossip_period * 7));

    let peer_messages = peer_messages.lock().unwrap();
    for instance in &instances {
        let my_messages = peer_messages.get(&instance.address().to_string()).unwrap();
        assert_eq!(all_messages.len(), my_messages.len());
        for update in &all_messages {
            assert!(my_messages.contains(update.digest()));
        }
    }

    for mut instance in instances {
        instance.shutdown();
    }
}
