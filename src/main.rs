#![feature(error_reporter)]

use std::net::SocketAddr;
use std::time::Duration;
use std::num::NonZero;

use clap::Parser;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use reqwest::Url;

mod aggregator;
mod rpc;

mod trees;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[arg(long, value_parser = parse_duration, default_value = "0.1")]
    period: Duration,

    #[arg(long, default_value = "127.0.0.1:3000")]
    bind: SocketAddr,

    #[arg(long, default_value = "256")]
    queue_depth: NonZero<usize>,

    #[arg(value_parser = parse_url)]
    upstream_url: Url,

    /// Human readable name for us
    #[arg(long)]
    our_name: Option<String>,

    /// Human readable name for the upstream calendar
    #[arg(long)]
    upstream_calendar_name: Option<String>,
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
    env_logger::init();

    let args = Args::parse();

    let (request_sender, request_receiver) = tokio::sync::mpsc::channel(args.queue_depth.into());

    tokio::task::spawn(aggregator::aggregator_task(request_receiver, args.period, args.upstream_url.clone()));

    // We create a TcpListener and bind it
    let listener = TcpListener::bind(args.bind).await?;

    log::info!("listening on {}", args.bind);

    // We start a loop to continuously accept incoming connections
    loop {
        let our_name = args.our_name.clone().unwrap_or(args.bind.to_string());
        let upstream_calendar_name = args.upstream_calendar_name.clone().unwrap_or(args.upstream_url.to_string());

        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        let request_sender = request_sender.clone();
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our RPC service
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, rpc::RPCService::new(
                        request_sender,
                        our_name.clone(),
                        upstream_calendar_name.clone(),
                        ))
                .await
            {
                log::debug!("Error serving connection: {}", std::error::Report::new(err).pretty(true));
            }
        });
    }
}
