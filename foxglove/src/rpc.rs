use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::Pin;

use http_body_util::{Full, Limited, BodyExt, LengthLimitError};
use hyper::http;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::Service;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use reqwest;

use crate::aggregator::StampRequest;

async fn do_get_root() -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!\n"))))
}

async fn do_post_digest(
    r: Request<hyper::body::Incoming>,
    req_sender: tokio::sync::mpsc::Sender<StampRequest>,
)
    -> Result<Response<Full<Bytes>>, Infallible>
{
    let digest_fut = Limited::new(r.into_body(), 64)
                             .collect();

    match digest_fut.await {
        Ok(digest) => {
            let digest = digest.to_bytes();

            let (req, timestamp_receiver) = dbg!(StampRequest::new(&digest));

            req_sender.send(req).await.unwrap();

            dbg!(timestamp_receiver.await);

            Ok(Response::new(Full::new(Bytes::from(""))))
            /*
            let client = reqwest::Client::new();
            let response = client.post("https://a.pool.opentimestamps.org/digest")
                                 .body(digest)
                                 .send()
                                 .await.unwrap();
            dbg!(&response);

            let proof = response.bytes().await.unwrap();
            dbg!(&proof);

            Ok(Response::new(Full::new(proof)))
            */
        },
        Err(e) => {
            match e.downcast::<LengthLimitError>() {
                Ok(e) => {
                    Ok(Response::builder()
                                .status(400)
                                .header(http::header::CONTENT_TYPE, "text/plain")
                                .body(Full::new(Bytes::from("digest too long\n")))
                                .unwrap())
                },
                Err(e) => {
                    unimplemented!("{:?}", e);
                }
            }
        },
    }
}

async fn serve_http_request(
    r: Request<hyper::body::Incoming>,
    digest_sender: tokio::sync::mpsc::Sender<StampRequest>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    dbg!(&r);
    match r.uri().path() {
        "/" => do_get_root().await,
        "/digest" if r.method() == http::Method::POST => do_post_digest(r, digest_sender).await,
        _ => {
            Ok(Response::builder()
                        .header(http::header::CONTENT_TYPE, "text/plain")
                        .header(http::header::CACHE_CONTROL, "public, max-age=3600")
                        .status(404)
                        .body(Full::new(Bytes::from("Not found\n")))
                        .unwrap())
        }
    }
}

pub struct RPCService {
    request_sender: tokio::sync::mpsc::Sender<StampRequest>,
}

impl RPCService {
    pub fn new(request_sender: tokio::sync::mpsc::Sender<StampRequest>) -> Self {
        Self { request_sender }
    }
}

impl Service<Request<hyper::body::Incoming>> for RPCService {
    type Response = Response<Full<Bytes>>;
    type Error = Infallible; //Box<dyn std::error::Error + Send + Sync + 'static>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync + 'static>>;

    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        Box::into_pin(Box::new(serve_http_request(req, self.request_sender.clone())))
    }
}
