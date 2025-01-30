use std::convert::Infallible;
use std::net::SocketAddr;

use http_body_util::{Full, Limited, BodyExt, LengthLimitError};
use hyper::http;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use reqwest;

mod aggregator;
mod rpc;

mod trees;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let period = std::time::Duration::from_millis(1000);
    let (request_sender, request_receiver) = tokio::sync::mpsc::channel(256);

    tokio::task::spawn(aggregator::aggregator_task(request_receiver, period));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        let request_sender = request_sender.clone();
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, rpc::RPCService::new(request_sender))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
