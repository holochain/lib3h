# lib3h_capnp_build

We can't expect developers working with lib3h or holochain to have the capnprotocol compiler tools installed on their system. This means we cannot rely on the more dynamic route of compiling the schemas in a build.rs file for lib3h_p2p_protocol. Instead, we will provide this tool. Anyone iterating on the p2p protocol or the transit encoding protocol can run this tool during the development cycle, and commit the resulting generated code into the repository.

## Usage

First make sure the capnp schema compiler binary is in your path.
If you built it manually, but didn't install, you may need to:

```
export PATH=/path/to/capnp/binary:$PATH
```

From the lib3h monorepo root:

```
cargo run -p lib3h_capnp_build
```

From the capnp_build crate directory:

```
cargo run
```

## Linux Usage

If you're on linux, and don't want to install capnp to your system, you can use the bash source script to download and build capnp, temporarily adding the binary path to your current shell:

```
source ./capnp-build-and-source.bash
```

note, you will need support packages like: `pkg-config autoconf automake libtoolmake` plus compiler/build-essential/etc.
