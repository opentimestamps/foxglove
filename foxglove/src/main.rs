use std::net::SocketAddr;
use std::time::Duration;

use clap::Parser;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use reqwest::Url;

mod aggregator;
mod rpc;

mod trees;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, value_parser = parse_duration, default_value = "0.1")]
    period: Duration,

    #[arg(value_parser = parse_url)]
    upstream_url: Url,
}

fn parse_duration(arg: &str) -> Result<Duration, std::num::ParseFloatError> {
    let seconds = arg.parse()?;
    Ok(Duration::from_secs_f64(seconds))
}

fn parse_url(arg: &str) -> Result<Url, Box<dyn std::error::Error + Send + Sync + 'static>> {
    Ok(Url::parse(arg)?)
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let (request_sender, request_receiver) = tokio::sync::mpsc::channel(256);

    tokio::task::spawn(aggregator::aggregator_task(request_receiver, args.period, args.upstream_url));

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
