# Presentation
This crate implements a gossip protocol that is configurable and generic with respect to the application. 
All aspects of the gossip protocol can be customized (push/pull, gossip period,...), and any data that can be represented in a binary format can broadcast to the network.

The overlay network between the peers is created using [Gossip-based Peer Sampling](https://infoscience.epfl.ch/record/109297/files/all.pdf). The configuration for peer sampling can also be customized.

# API
The gossiping functionalities are provided by the `GossipService` struct:
 - `start` starts the gossip protocol on the node
 - `submit` broadcasts an update to the network
 - `shutdown` terminates the gossip protocol on the node

# Initialization
To join an existing network, a new node must connect to at least one existing peer to learn about other peers. 
This is done by providing the `start` method with a closure that returns a list of `Peer`. The peer(s) can be hardcoded or retrieved via any other means inside the closure.

If no `Peer` is returned by the closure, the node will wait for connections from other peers.

# Receiving updates from the network
Updates broadcast by other peers must be delivered to the application layer. 
To this end, the `start` method also requires a struct implementing the `UpdateHandler` trait to handle `Update` messages received from other peers.

# Example
```
// node address
let address = "127.0.0.1:9000";

// existing peer(s) in the network
let existing_peers = || Some(vec![ Peer::new("127.0.0.1:9001".to_owned()) ]);

// create and start the service
let mut gossip_service = GossipService::new_with_defaults(address.parse().unwrap());
gossip_service.start(Box::new(existing_peers), Box::new(MyUpdateHandler::new()))?;

// submit a message
gossip_service.submit("Some random message".as_bytes().to_vec())?;

// shutdown the gossip protocol
gossip_service.shutdown();
```