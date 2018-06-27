# lib3h

The Happy HedgeHog p2p and distributed hash table rust library.

WIP!! Everything in here is subject to change!

## What is it?

This is intended as a functional stub for the p2p networking layer for the holochain rust rewrite.

### MVP functionality required:

- Ability for any node to query a list of all peers on the network.
- Ability for any node to message any peer on the network.
- Ability to publish DHT values.
- Ability to query DHT values.

### Nice to have, but not in MVP scope:

- Ability to route messages to nodes behind a NAT.
- Solid encryption / anonymity.
- Gossip and message efficiency.
- DHT distribution efficiency.

## How does the p2p layer work?

### Node identification

Node IDs are a 256 bit hash of all public keys associated with an identity. Right now this includes a single rsa4096 key, but rsa will likely go away in favor of better alternatives.

### Node types

#### Routing Nodes

Initially there will just be routing nodes. Routing nodes are required to be accessible to tcp connections.

#### Leaf Nodes

If we tackle the NAT accessibility, then leaf nodes that are not directly accessible will be allowed to connect and receive messages that are routed to them.

### Flood Data

A set of information, including node public keys, ids, u32_tags (see DHT info below), current routing connections, and some optional light metadata will be flood propagated to all peers on the network. This will allow each node to determine routing and dht coverage in an agent-centric manner.

### Node Communication

Nodes will communicate via encrypted ephemeral http connections. Short term-sessions will be established through some TBD handshake protocol to agree on shared secret keys (Diffie-Hellman?). Nodes will attempt to keep sessions with some configurable number of other nodes distributed as evenly as possible through the current calculated DHT bucket space (see below). They will also slowly rotate connections to different nodes.

## How does the DHT layer work?

Rather than using Kademlia, or another similar approach, this library functions more like a classic hash structure, just distributed across the connected p2p nodes.

### the u32_tag

Every node or bit of data on the network is uniquely identified by a 256 bit hash. This hash is interpreted as an unsigned big integer (little-endian) then modulo down to fit into an unsigned 32 bit integer. This should maintain the distribution quality of the origin hashing algorithm.

### DHT bucket algorithm

Starting with 1 and doubling every iteration, the u32_tag space is divided up, and the node-count for each division is calculated. If there are more than N number of nodes depending on the redundancy settings, (Let us say 4 for example purposes) then we proceed to the next iteration, otherwise we have determined the DHT bucket count.

For the first iteration, the only consideration is the total number of nodes on the network. If there are > 4 nodes, we move on to iteration with bucket count 2.

For the second iteration, we see how many nodes fit into the 0 – 2147483647 range, and how many fit into the 2147483647 – 4294967295 range (u32 max / 2). If both these buckets have more than 4 nodes, continue onto the next iteration, etc.

Every node individually runs their own bucket algorithm, and may come to different answers depending on node visibility.

At the end of the algorithm, every node should know their current bucket count, and which bucket they fit into.

### DHT value assignment

Every value assigned to the DHT also has a u32_tag. Nodes that have self-assigned themselves a bucket range that includes the u32_tag of this piece of data, will be responsible for storing and responding to queries for this piece of data.

### DHT re-hashing

This bucket algorithm provides some stability that should make re-hashing less onerous. If a node re-evaluates the p2p network and decides to double the bucket count, then it is effectively deciding to forget half of the data it is currently indexing. Conversely, if a node decides to halve the bucket count, they will already be indexing half the data they need to be, they will just need to fetch the other half from the network.
