use gossip::GossipService;

mod common;

#[test]
fn peer_sampling_smoke_test() {
    use gossip::{GossipConfig, PeerSamplingConfig, MonitoringConfig, Peer};

    common::configure_logging(log::LevelFilter::Warn).unwrap();

    // algorithm parameters
    let gossip_period = 2000;

    let sampling_period = 2;
    let sampling_deviation = 2;
    let push = true;
    let pull = true;
    let t = 5;
    let d = 5;
    let c = 4;
    let h = 1;
    let s = 2;

    let monitoring_config = MonitoringConfig::new(true, "127.0.0.1:8080".to_owned(), "/peers".to_owned(), "/updates".to_owned());

    let peers_per_protocol = 10;
    let mut instances = vec![];

    // create first peer with no contact peer
    let init_address = "127.0.0.1:9000";
    // no contact peer for first node
    let no_peer_handler = Box::new(move|| { None });

    // create and initiate the peer sampling service
    let mut service = GossipService::new(
        init_address.parse().unwrap(),
        PeerSamplingConfig::new(true, true, sampling_period, sampling_deviation, c, h, c),
        GossipConfig::new(true, true, init_address.parse().unwrap(), gossip_period),
        Some(monitoring_config.clone())
    );
    service.start(no_peer_handler);
    instances.push(service);

    // create peers using IPv4 addresses
    let mut port = 9001;
    for _ in 1..peers_per_protocol {
        // peer socket address
        let address = format!("127.0.0.1:{}", port);
        // closure for retrieving the address of the first contact peer
        let init_handler = Box::new(move|| { Some(vec![Peer::new(init_address.to_owned())]) });

        // create and initiate the gossip service
        let mut ipv4_service = GossipService::new(
            address.parse().unwrap(),
            PeerSamplingConfig::new(true, true, sampling_period, sampling_deviation, c, h, c),
            GossipConfig::new(true, true, address.parse().unwrap(), gossip_period),
            Some(monitoring_config.clone())
        );
        ipv4_service.start(init_handler);
        instances.push(ipv4_service);

        port += 1;
    }
/*
    // create peers using IPv6 addresses
    for _ in 1..peers_per_protocol {
        // peer socket address
        let address = format!("[::1]:{}", port);
        // configuration
        let config = Config::new(address.parse().unwrap(), push, pull, t, d, c, h, s, Some(monitoring_config.clone()));
        // closure for retrieving the address of the first contact peer
        let init_handler = Box::new(move|| { Some(vec![Peer::new(init_address.to_owned())]) });

        // create and initiate the peer sampling service
        let mut ipv6_service = PeerSamplingService::new(config);
        ipv6_service.init(init_handler);
        instances.push(ipv6_service);

        port += 1;
    }
*/
    std::thread::sleep(std::time::Duration::from_secs(11));

    //assert!(&instances[0].get_peer().is_some());

    for mut instance in instances {
        instance.shutdown();
    }
}
