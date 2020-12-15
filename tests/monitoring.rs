use gossip::UpdateExpirationMode;

mod common;

#[test]
fn peer_sampling_smoke_test() {
    use rand::Rng;
    use gossip::{GossipConfig, PeerSamplingConfig, MonitoringConfig, Peer, GossipService};

    common::configure_logging(log::LevelFilter::Warn).unwrap();

    // algorithm parameters
    let gossip_period = 400;
    let gossip_deviation = 400;

    let sampling_period = 2000;
    let sampling_deviation = 2000;
    let push = true;
    let pull = true;
    let t = 5;
    let d = 5;
    let c = 4;
    let h = 1;
    let s = 2;

    let monitoring_config = MonitoringConfig::new(true, "127.0.0.1:8080".to_owned(), "/peers".to_owned(), "/updates".to_owned());

    let peers_per_protocol = 20;
    let mut instances = vec![];

    // create first peer with no contact peer
    let init_address = "127.0.0.1:9000";
    // no contact peer for first node
    let no_peer_handler = Box::new(move|| { None });

    // create and initiate the peer sampling service
    let mut service = GossipService::new(
        init_address.parse().unwrap(),
        PeerSamplingConfig::new_with_deviation(true, true, sampling_period, sampling_deviation, c, h, c),
        GossipConfig::new_with_deviation(true, true, gossip_period, gossip_deviation, UpdateExpirationMode::None),
        Some(monitoring_config.clone())
    );
    service.start(no_peer_handler, Box::new(common::TextMessageListener::new(init_address.to_owned())));
    instances.push(service);

    let mut port = 9001;

    // create peers using IPv4 addresses
    for _ in 0..peers_per_protocol {
        // peer socket address
        let address = format!("127.0.0.1:{}", port);
        // closure for retrieving the address of the first contact peer
        let init_handler = Box::new(move|| { Some(vec![Peer::new(init_address.to_owned())]) });

        // create and initiate the gossip service
        let mut ipv4_service = GossipService::new(
            address.parse().unwrap(),
            PeerSamplingConfig::new_with_deviation(push, pull, sampling_period, sampling_deviation, c, h, c),
            GossipConfig::new_with_deviation(push, pull, gossip_period, gossip_deviation, UpdateExpirationMode::None),
            Some(monitoring_config.clone())
        );
        ipv4_service.start(init_handler, Box::new(common::TextMessageListener::new(address.to_owned())));
        instances.push(ipv4_service);

        port += 1;
    }

    // create peers using IPv6 addresses
    for _ in 0..peers_per_protocol {
        // peer socket address
        let address = format!("[::1]:{}", port);
        // closure for retrieving the address of the first contact peer
        let init_handler = Box::new(move|| { Some(vec![Peer::new(init_address.to_owned())]) });

        // create and initiate the gossip service
        let mut ipv6_service = GossipService::new(
            address.parse().unwrap(),
            PeerSamplingConfig::new_with_deviation(push, pull, sampling_period, sampling_deviation, c, h, c),
            GossipConfig::new_with_deviation(push, pull, gossip_period, gossip_deviation, UpdateExpirationMode::None),
            Some(monitoring_config.clone())
        );
        ipv6_service.start(init_handler, Box::new(common::TextMessageListener::new(address.to_owned())));
        instances.push(ipv6_service);

        port += 1;
    }

    std::thread::sleep(std::time::Duration::from_secs(11));

    for i in 0..40 {
        let selected_peer = rand::thread_rng().gen_range(0, instances.len());
        instances[selected_peer].submit(format!("MSG_ID_{}", i).as_bytes().to_vec());
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    std::thread::sleep(std::time::Duration::from_secs(20));
    log::error!("-----------------------");
    log::error!("-----------------------");
    log::error!("-----------------------");
    log::error!("SHUTDOWN");
    log::error!("SHUTDOWN");
    log::error!("SHUTDOWN");
    log::error!("-----------------------");
    log::error!("-----------------------");
    log::error!("-----------------------");
    std::thread::sleep(std::time::Duration::from_secs(5));

    for mut instance in instances {
        instance.shutdown();
    }
}
