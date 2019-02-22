extern crate futures;
extern crate tokio;
extern crate tokio_core;
extern crate hyper;
extern crate crypto;
extern crate rand;
extern crate data_encoding;
extern crate env_logger;
extern crate hyper_tls;


#[macro_use]
extern crate log;
extern crate opentimestamps;

use std::time::Duration;

pub mod server;
pub mod merkle;
pub mod timer;
pub mod timestamp;

pub trait Millis {
    fn as_millis(&self) -> f64;
}
impl Millis for Duration {
    fn as_millis(&self) -> f64 {
        self.as_secs() as f64 * 1000.0 +
            f64::from(self.subsec_nanos()) / 1_000_000.0
    }
}
