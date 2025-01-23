use std::convert::Infallible;

use tokio;
use hyper::Uri;

#[derive(Debug)]
pub struct LinearTimestamp([u8; 32]);

#[derive(Debug)]
pub struct StampRequestError {
}

#[derive(Debug)]
pub struct StampRequest {
    digest: [u8; 32],
    reply: tokio::sync::oneshot::Sender<Result<LinearTimestamp, StampRequestError>>,
}

impl StampRequest {
    pub fn new(digest: &[u8]) -> (Self, tokio::sync::oneshot::Receiver<Result<LinearTimestamp, StampRequestError>>) {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        (Self {
            digest: [0; 32],
            reply: sender,
         },
         receiver)
    }
}

pub async fn aggregate_requests(requests: Vec<StampRequest>) {

    /*
            let client = reqwest::Client::new();
            let response = client.post("https://a.pool.opentimestamps.org/digest")
                                 .body(digest)
                                 .send()
                                 .await.unwrap();
            dbg!(&response);

            let proof = response.bytes().await.unwrap();
            dbg!(&proof);
    */

    for request in requests.into_iter() {
        dbg!(request.reply.send(Ok(LinearTimestamp(request.digest))));
    }
}

pub async fn aggregator_task(
    mut request_mpsc: tokio::sync::mpsc::Receiver<StampRequest>,
    mut period: tokio::time::Duration,
) -> Result<(), Infallible>
{
    let mut interval = tokio::time::interval(period);

    while !request_mpsc.is_closed() {
        print!("{:?}\n", interval.tick().await.into_std());

        let mut requests: Vec<StampRequest> = vec![];
        while let Ok(request) = dbg!(request_mpsc.try_recv()) {
            requests.push(request);
        }

        let _ = tokio::spawn(aggregate_requests(requests));
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aggregate_requests() {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let req = StampRequest {
            digest: [0; 32],
            reply: sender,
        };
        aggregate_requests(vec![req]).await;

        dbg!(receiver.await.unwrap());
    }

    #[tokio::test]
    async fn test_aggregator() -> Result<(), Box<dyn std::error::Error>> {
        let period = std::time::Duration::from_millis(100);
        let (sender, request_mpsc) = tokio::sync::mpsc::channel(128);
        let task = aggregator_task(request_mpsc, period);

        let (req_reply, stamp_recv) = tokio::sync::oneshot::channel();
        sender.send(StampRequest {
            digest: [0; 32],
            reply: req_reply,
        }).await;

        //task.await.unwrap();

        //dbg!(stamp_recv.await.unwrap());
        Ok(())
    }
}
