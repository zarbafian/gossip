use gossip::{Update, UpdateHandler, UpdateExpiration};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod common;

pub struct MapUpdatingListener {
    id: String,
    map: Arc<Mutex<HashMap<String, Vec<String>>>>,
}
impl MapUpdatingListener {
    pub fn new(id: String, map: Arc<Mutex<HashMap<String, Vec<String>>>>) -> Self {
        MapUpdatingListener{
            id,
            map,
        }
    }
}
impl UpdateHandler for MapUpdatingListener {
    fn on_update(&self, update: Update) {
        self.map.lock().unwrap().entry(self.id.clone()).or_insert(Vec::new()).push(update.digest().clone());
    }
}

#[test]
fn all_updates_received() {
    use rand::Rng;
    use gossip::{GossipConfig, PeerSamplingConfig, MonitoringConfig, Peer, GossipService};

    common::configure_logging(log::LevelFilter::Warn).unwrap();

    // messages received by each peer
    let mut peer_messages = Arc::new(Mutex::new(HashMap::new()));

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

    let update_expiration = UpdateExpiration::Count(2);

    let monitoring_config = MonitoringConfig::new(true, "127.0.0.1:8080".to_owned(), "/peers".to_owned(), "/updates".to_owned());

    let peer_count = 40;
    let mut instances = vec![];

    // create first peer with no contact peer
    let init_peer = "127.0.0.1:9000";
    // no contact peer for first node
    let no_peer_handler = Box::new(move|| { None });

    // create and initiate the peer sampling service
    let mut service = GossipService::new(
        init_peer.parse().unwrap(),
        PeerSamplingConfig::new_with_params(push, pull, sampling_period, sampling_deviation, c, h, c),
        GossipConfig::new(push, pull, init_peer.parse().unwrap(), gossip_period, gossip_deviation, update_expiration.clone()),
        Some(monitoring_config.clone())
    );
    service.start(no_peer_handler, Box::new(MapUpdatingListener::new(init_peer.to_owned(), Arc::clone(&peer_messages))));
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
            PeerSamplingConfig::new_with_params(push, pull, sampling_period, sampling_deviation, c, h, c),
            GossipConfig::new(push, pull, address.parse().unwrap(), gossip_period, gossip_deviation, update_expiration.clone()),
            Some(monitoring_config.clone())
        );
        ipv4_service.start(init_handler, Box::new(MapUpdatingListener::new(address.clone(), Arc::clone(&peer_messages))));
        instances.push(ipv4_service);

        port += 1;
    }

    std::thread::sleep(std::time::Duration::from_secs(10));
    log::error!("--------------------------");
    log::error!("SAMPLING SHOULD BE READY");
    log::error!("--------------------------");

    let message_count = 40;
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

    std::thread::sleep(std::time::Duration::from_secs(2));
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