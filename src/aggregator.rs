use std::convert::Infallible;
use std::sync::Arc;

use bitcoin_hashes::Sha256;
use rand;
use reqwest::{StatusCode, Url};

use tokio;

use crate::trees::{Op, hash_tree};

#[derive(Debug)]
pub struct LinearTimestamp {
    nonce: [u8; 8],
    ops: Vec<Op>,
    proof: Vec<u8>,
}

impl LinearTimestamp {
    pub fn serialize(&self) -> Box<[u8]> {
        let mut r = vec![];
        r.push(0xf0); // append
        r.push(8); // 8 byte nonce
        r.extend_from_slice(&self.nonce);
        r.push(0x08); // sha256

        for ops in self.ops.iter() {
            match ops {
                Op::Sha256 => {
                    r.push(0x08); // sha256
                },
                Op::Append(digest) => {
                    r.push(0xf0);
                    r.push(32); // 32 bytes
                    r.extend_from_slice(digest);
                },
                Op::Prepend(digest) => {
                    r.push(0xf1);
                    r.push(32); // 32 bytes
                    r.extend_from_slice(digest);
                },
            }
        }

        r.extend_from_slice(&self.proof);

        r.into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StampRequestError {
    #[error("upstream aggregator failed: {0}")]
    Upstream(#[from] reqwest::Error),

    #[error("upstream aggregator returned bad status code: {0}")]
    BadStatus(StatusCode)
}

#[derive(Debug)]
pub struct StampRequest {
    nonce: [u8; 8],
    digest: [u8; 32],
    reply: tokio::sync::oneshot::Sender<Result<LinearTimestamp, Arc<StampRequestError>>>,
}

impl StampRequest {
    pub fn new(digest: &[u8]) -> (Self, tokio::sync::oneshot::Receiver<Result<LinearTimestamp, Arc<StampRequestError>>>) {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        let nonce: [u8; 8] = rand::random();
        (Self {
            digest: Sha256::hash_byte_chunks(&[digest, &nonce]).to_byte_array(),
            nonce,
            reply: sender,
         },
         receiver)
    }
}

pub async fn aggregate_requests(requests: Vec<StampRequest>, upstream_url: Url) {
    let digests: Vec<[u8; 32]> = requests.iter().map(|req| req.digest).collect();

    let (ops, tip_digest) = hash_tree(&digests);

    let client = reqwest::Client::new();

    match (async || -> Result<_, StampRequestError> {
        let response = client.post(upstream_url)
                             .body(Vec::from(tip_digest))
                             .send()
                             .await?;
        if response.status() == StatusCode::OK {
            let proof = response.bytes().await?;
            log::debug!("got {} bytes of proof from upstream", proof.len());
            Ok(proof)
        } else {
            Err(StampRequestError::BadStatus(response.status()))
        }
    })().await {
        Ok(proof) => {
            for (request, ops) in requests.into_iter().zip(ops.into_iter()) {
                let stamp = LinearTimestamp {
                    nonce: request.nonce,
                    ops,
                    proof: proof.clone().into(),
                };

                let _ = request.reply.send(Ok(stamp));
            }
        }
        Err(err) => {
            log::error!("{}", &err);
            let err = Arc::new(err);
            for request in requests.into_iter() {
                let _ = request.reply.send(Err(Arc::clone(&err)));
            }
        },
    }
}

pub async fn aggregator_task(
    mut request_mpsc: tokio::sync::mpsc::Receiver<StampRequest>,
    period: tokio::time::Duration,
    upstream_url: Url,
) -> Result<(), Infallible>
{
    let mut interval = tokio::time::interval(period);

    while !request_mpsc.is_closed() {
        interval.tick().await;

        let mut requests: Vec<StampRequest> = vec![];
        while let Ok(request) = request_mpsc.try_recv() {
            requests.push(request);
        }

        if requests.len() > 0 {
            log::info!("got {} requests", requests.len());
            let _ = tokio::spawn(aggregate_requests(requests, upstream_url.clone()));
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aggregate_requests() {
        let url = Url::parse("https://a.pool.opentimestamps.org/digest").unwrap();

        let (sender, receiver) = tokio::sync::oneshot::channel();
        let req = StampRequest {
            nonce: [0; 8],
            digest: [0; 32],
            reply: sender,
        };
        aggregate_requests(vec![req], url).await;

        receiver.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_aggregator() -> Result<(), Box<dyn std::error::Error>> {
        let url = Url::parse("https://a.pool.opentimestamps.org/digest").unwrap();

        let period = std::time::Duration::from_millis(100);
        let (sender, request_mpsc) = tokio::sync::mpsc::channel(128);
        let _task = aggregator_task(request_mpsc, period, url);

        let (req_reply, _stamp_recv) = tokio::sync::oneshot::channel();
        sender.send(StampRequest {
            nonce: [0; 8],
            digest: [0; 32],
            reply: req_reply,
        }).await.unwrap();

        Ok(())
    }
}
