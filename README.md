# lib3h

The lib3h p2p communication rust library.

WIP/POC!! Everything in here is subject to change!

## What is it?

This is intended as a functional stub for the p2p networking layer for the holochain rust rewrite.

## What does 3h stand for?

Nothin' ... pick some acronym with 3 h-words that makes it easy to remember.
- Harrowing Holographic Hat
- Happy HedgeHog
- Hilariously Humble Hippo

## Crypto

Using libsodium for everything to start: rng, key exchange, symmetric encryption, and eventually signing, hashing, and password hashing for private key encryption.

## Phases

We are currently in the proof-of-concept stub phase "0".

### Phase 0 - "Stub" - the fully connected network...

This phase is intended to bootstrap the rest of the holochain rust project. It provides the functionality of nodes being able to communicate with each other.

- The code is not pretty, efficient, or final.
- The model is a non-scalable fully connected network.

### Phase 1 - "IP Discovery" - the interim transport solution...

This phase will provide a minimally connected p2p network that allows discovery of other nodes on the network, but will rely on ip routing to actually message those nodes.

- The code is becoming more production ready.
- The transport layer is abstracted, but only provides a tcp/ip transport, and a dummy transport for unit testing.
- The model is scalable and ready to be used in production, even if it isn't our final solution.

### Phase 2 - "??" - the transport agnostic custom routing solution...

We have a shared discovery space, we can know what nodes are currently connected to each other. We should be able to route information between nodes without creating direct connecitions.

- Everything TBD.

## Building

For sodiumoxide, libsodium-sys, you need some packages:

```shell
sudo apt-get install pkg-config libsodium clang-6.0 libclang-6.0-dev
```

My particular disto didn't provide a new-enough libsodium, so I've included a script that will prep a local build, and set up the environment so cargo will use that one, you simply need to source the helper script:

```shell
source ./prep-3rd-party.bash
```

This will build libsodim 1.0.16 the first time you run it, and use the cached build from then on. Note, you'll still need to source the helper script for every new terminal you open.

## Example

try out the three node babble example:

```shell
cargo run -p babble
```

It will spin up three nodes, each of which will connect to or discover the other two nodes, and then begins sending messages between them.
