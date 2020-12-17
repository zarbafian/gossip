# Presentation
This crate implements a gossip protocol that is configurable and generic with respect to the application. 
All aspects of the gossip protocol can be customized (push/pull, gossip period,...), and any data that can be represented in a binary format can broadcast to the network.

The overlay network between the peers is created using [Gossip-based Peer Sampling](https://infoscience.epfl.ch/record/109297/files/all.pdf) [[1]]. The configuration for peer sampling can also be customized.

## Gossip algorithms
Gossip algorithms enable the propagation of information in a network. It is achieved by nodes exchanging their knowledge. 
At each round, a node selects a peer at random in the network and sends him the state of his knowledge. The peer responds with its own view, 
and each peer is able to request the parts of the view that are missing on his side.

## Gossip-based Peer Sampling
In large distributed systems that use gossip protocols to broadcast information to the network, peers should be selected at random in the network. In theory, this requires a knowledge of all the participating nodes. Because peers are constantly coming and leaving, this can be impractical. Gossip-based Peer Sampling is an algorithm by Jelasity[[1]] et al. that solves the random peer selection problem by building a local view of the network that simulates a random view. 
The algorithm consists of rounds of `push` and/or `pull` when peers exchange their views of the network. 
During each round a node selects a peer at random and either sends him (i.e. push) a selection of peers inside its view (if `push` is enabled) or an empty view to trigger a pull (if `push` is disabled). The selected node will process the view received (possibly empty), and respond with its own view if `pull` is enabled.

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
```rust
// node address
let address = "127.0.0.1:9000";

// existing peer(s) in the network
let existing_peers = || Some(vec![ Peer::new("127.0.0.1:9001".to_owned()) ]);

// create and start the service
let mut gossip_service = GossipService::new_with_defaults(address.parse().unwrap());
gossip_service.start(Box::new(existing_peers), Box::new(MyUpdateHandler::new()))?;

// submit a message
gossip_service.submit("Some random message".as_bytes().to_vec())?;

// shutdown the gossip protocol on exit
//gossip_service.shutdown();
```

[1]: https://infoscience.epfl.ch/record/109297/files/all.pdf
[[1]]: M. Jelasity, S. Voulgaris, R. Guerraoui, A.-M. Kermarrec, M. van Steen, Gossip-based Peer Sampling, 2007