# Foxglove

Foxglove is a high performance OpenTimestamps aggregator.

Every interval (e.g. 100ms) it builds a merkle tree of all incoming timestamp
requests, and then forwards the tip digest to an upstream aggregator/calendar.
When the upstream aggregator replies, all pending requests are responded to
with the completed timestamp. Thus it allows for horizontal scaling of
timestamp creation. Foxglove is entirely stateless, and does not save anything
to disk.

It is written in Rust, using the Tokio and Hyper crates. It doesn't actually
use the rust-opentimestamps crate yet, as it trusts the upstream aggregator
fully without actually validating that upstream timestamps are structually
valid; the steps in the merkle tree are seralized "by hand".

# Status

At the moment, Foxglove's basic functionality works and is being experimentally
used in production on the Alice calendar. But it is quite primitive, without
logging, proper error handling, tests, etc.
