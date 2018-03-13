extern crate futures;
extern crate tokio;
extern crate tokio_core;
extern crate hyper;
extern crate crypto;
extern crate rand;

use std::time::Duration;

pub mod client;
pub mod server;
pub mod merkle;


pub trait Millis {
    fn as_millis(&self) -> f64;
}
impl Millis for Duration {
    fn as_millis(&self) -> f64 {
        self.as_secs() as f64 * 1000.0 +
            self.subsec_nanos() as f64 / 1000000.0
    }
}