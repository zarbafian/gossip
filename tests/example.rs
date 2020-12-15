use gossip::{GossipService, Peer};
use std::error::Error;

mod common;

#[test]
fn example() -> Result<(), Box<dyn Error>>{
    // node address
    let address = "127.0.0.1:9000";

    // existing peer(s) in the network
    let existing_peers = || Some(vec![ Peer::new("127.0.0.1:9001".to_owned()) ]);

    // create and start the service
    let mut gossip_service = GossipService::new_with_defaults(address.parse().unwrap());
    gossip_service.start(Box::new(existing_peers), Box::new(common::TextMessageListener::new("John".to_owned())))?;

    // submit a message
    gossip_service.submit("Some random message".as_bytes().to_vec())?;

    // shutdown the gossip protocol
    gossip_service.shutdown();
    Ok(())
}