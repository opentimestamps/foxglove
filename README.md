# Foxglove

Foxglove is an high performance OpenTimestamps requests aggregator.

It works by aggregating requests received in a time slice (eg. 100ms), building a merkle tree and making just one
request to a back end OpenTimestamps calendar. When the calendar replies all the pending requests are served.
The aggregator is persistent-stateless, does not save anything to disk.
In the future it will be possible to achieve even more scaling by using more aggregator per single Calendar server, thus
scaling horizontally.

To achieve high performance it is entirely futures-based and blocking-free code.

## Todo

* Do proper load testing and performance measurements
* Move to a multi-threaded event loop when [tokio](https://tokio.rs/blog/2018-03-tokio-runtime/) stabilize
* Run clippy linter and solve any problem
* Proper initialize logging system and better handle logging (with proper error message in map_err) removing println!
* Handling errors in map_err
* Partially building the merkle tree at every requests instead of building it entirely at the end
* Evaluate bottlenecks and if the first limit could be the number of open file descriptor by the os, eventually configure the system to allow more connections


