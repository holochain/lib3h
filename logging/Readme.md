# Lib3h Logger #

This logger use the Rust crate [log](https://crates.io/crates/log) in the back-end and [fern](https://crates.io/crates/fern) for its **executable** part.

## Implementation choice ##

The first implementation of this logger used [slog](https://crates.io/crates/slog) and more specifically:
- [slog-async](https://crates.io/crates/slog-async) for its asynchronous capability, which is a must have regarding the potential high throughput of **Lib3h** (the networking library of [Holochain](https://github.com/holochain/holochain-rust))

- Along with [slog-term](https://crates.io/crates/slog-term), to log in the terminal
- And [slog-scope](https://crates.io/crates/slog-scope) to be able to define a global logger in order to skip the burden of passing around a local one and manually managing it.

But it turned out to be **not** advised by the developers to use [slog-scope](https://crates.io/crates/fern) in library, so we switch to [fern](https://crates.io/crates/fern) instead.

[fern](https://crates.io/crates/fern) gives us a way more flexible logger in term of configuration(color, format, log level), which was required by the different teams.

## Implementation detail ##

The logger is currently implemented using its own thread in order to be asynchronous and can experience some annoyance especially during shutdown. This is why we give it a bit of extra time during drop.