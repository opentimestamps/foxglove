use std::pin::Pin;

use http_body_util::{Full, Limited, BodyExt, LengthLimitError};
use hyper::http;
use hyper::body::Bytes;
use hyper::service::Service;
use hyper::{Request, Response};
use http::status::StatusCode;

use crate::aggregator::StampRequest;

fn do_get_root(our_name: String, upstream_name: String) -> Response<Full<Bytes>> {
    let body = format!(
"<html>
<head>
    <title>OpenTimestamps Aggregator</title>
    <link rel=\"icon\" type=\"image/x-icon\" href=\"/favicon.ico\">
</head>
<body>
This is the <a href=\"https://opentimestamps.org\">OpenTimestamps</a> aggregator {}, aggregating timestamp requests for the upstream calendar server {}
</body>
</html>
",
        our_name,
        upstream_name
    );
    Response::builder()
             .status(StatusCode::OK)
             .header(http::header::CONTENT_TYPE, "text/html")
             .body(Full::new(Bytes::from(body)))
             .unwrap()
}

fn do_get_favicon() -> Response<Full<Bytes>> {
    Response::builder()
             .status(StatusCode::OK)
             .header(http::header::CONTENT_TYPE, "image/vnd.microsoft.icon")
             .body(Full::new(Bytes::from_static(include_bytes!("favicon.ico"))))
             .unwrap()
}

async fn do_post_digest(
    r: Request<hyper::body::Incoming>,
    req_sender: tokio::sync::mpsc::Sender<StampRequest>,
)
    -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>>
{
    let digest_fut = Limited::new(r.into_body(), 64)
                             .collect();

    match digest_fut.await {
        Ok(digest) => {
            let digest = digest.to_bytes();

            let (req, timestamp_receiver) = StampRequest::new(&digest);
            req_sender.send(req).await?;

            match timestamp_receiver.await? {
                Ok(stamp) => {
                    let stamp = stamp.serialize();
                    Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(http::header::CONTENT_TYPE, "application/vnd.opentimestamps.v1")
                                .body(Full::new(Bytes::from(stamp)))
                                .unwrap())
                },
                Err(err) => {
                    // FIXME: is having urls here potentially a security risk?
                    let body = format!("internal error: {}\n", &err);
                    Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(http::header::CONTENT_TYPE, "text/plain")
                                .body(Full::new(Bytes::from(body)))
                                .unwrap())
                },
            }
        },
        Err(e) => {
            match e.downcast::<LengthLimitError>() {
                Ok(_) => {
                    Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST) // should actually be 413 Payload Too Large
                                .header(http::header::CONTENT_TYPE, "text/plain")
                                .body(Full::new(Bytes::from("digest too long\n")))
                                .unwrap())
                },
                // FIXME: what exactly does an error here mean?
                Err(e) => Err(e),
            }
        },
    }
}

async fn serve_http_request(
    r: Request<hyper::body::Incoming>,
    digest_sender: tokio::sync::mpsc::Sender<StampRequest>,
    our_name: String,
    upstream_name: String,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    log::debug!("{:?}", r);
    match (r.method(), r.uri().path()) {
        (&http::Method::GET,  "/")            => Ok(do_get_root(our_name, upstream_name)),
        (&http::Method::GET,  "/favicon.ico") => Ok(do_get_favicon()),
        (&http::Method::POST, "/digest")      => Ok(do_post_digest(r, digest_sender).await?),
        _ => { // FIXME: distinguish methods being invalid (GET-vs-POST) and not found
            Ok(Response::builder()
                        .header(http::header::CONTENT_TYPE, "text/plain")
                        .header(http::header::CACHE_CONTROL, "public, max-age=3600")
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::new(Bytes::from("Not found\n")))?)
        }
    }
}

pub struct RPCService {
    request_sender: tokio::sync::mpsc::Sender<StampRequest>,
    our_name: String,
    upstream_calendar_name: String,
}

impl RPCService {
    pub fn new(request_sender: tokio::sync::mpsc::Sender<StampRequest>,
               our_name: String,
               upstream_calendar_name: String,
               ) -> Self {
        Self { request_sender, our_name, upstream_calendar_name }
    }
}

impl Service<Request<hyper::body::Incoming>> for RPCService {
    type Response = Response<Full<Bytes>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync + 'static>>;

    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        Box::into_pin(Box::new(
                serve_http_request(
                    req,
                    self.request_sender.clone(),
                    self.our_name.clone(),
                    self.upstream_calendar_name.clone(),
                )))
    }
}
