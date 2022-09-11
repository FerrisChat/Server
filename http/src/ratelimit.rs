use crate::response::{Error, Response};

use axum::{
    body::Body,
    extract::ConnectInfo,
    headers::HeaderMap,
    http::{header::FORWARDED, Request, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
};
use dashmap::DashMap;
use forwarded_header_value::{ForwardedHeaderValue, Identifier};
use tower::{Layer, Service};

use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::time::Instant;

#[derive(Clone, Debug, Hash)]
pub struct Bucket {
    pub count: u16,
    pub reset: Instant,
}

impl Bucket {
    #[must_use]
    pub const fn new(count: u16, reset: Instant) -> Self {
        Self { count, reset }
    }
}

#[derive(Debug)]
pub struct Ratelimit<S> {
    inner: S,
    rate: u16,
    per: u16,
    buckets: DashMap<IpAddr, Bucket>,
}

impl<S> Ratelimit<S> {
    pub fn new(service: S, rate: u16, per: u16) -> Self {
        Self {
            inner: service,
            rate,
            per,
            buckets: DashMap::new(),
        }
    }

    fn insert_headers(rate: u16, per: u16, headers: &mut HeaderMap) {
        headers.insert("X-RateLimit-Limit", rate.to_string().parse().unwrap());
        headers.insert("X-RateLimit-Per", per.to_string().parse().unwrap());
    }

    #[allow(clippy::cast_lossless)]
    fn handle_ratelimit(&mut self, headers: &HeaderMap, ip: IpAddr) -> Result<(), AxumResponse> {
        let mut bucket = self
            .buckets
            .entry(ip)
            .or_insert_with(|| Bucket::new(self.rate, Instant::now()));
        let bucket = bucket.value_mut();

        if bucket.reset > Instant::now() {
            let retry_after = bucket.reset.duration_since(Instant::now());

            let mut response = Response(
                StatusCode::TOO_MANY_REQUESTS,
                Error::Ratelimited {
                    retry_after: retry_after.as_secs_f32(),
                    ip: ip.to_string(),
                    rate: self.rate,
                    per: self.per,
                    message: format!(
                        "You are being rate limited. Try again in {:?}.",
                        retry_after,
                    ),
                },
            )
            .promote(headers)
            .into_response();

            let headers = response.headers_mut();
            Self::insert_headers(self.rate, self.per, headers);
            headers.insert(
                "X-RateLimit-Remaining",
                bucket.count.to_string().parse().unwrap(),
            );
            headers.insert(
                "Retry-After",
                retry_after.as_secs_f32().to_string().parse().unwrap(),
            );

            return Err(response);
        }

        bucket.count -= 1;
        if bucket.count == 0 {
            bucket.count = self.rate;
            bucket.reset = Instant::now() + Duration::from_secs(self.per as u64);
        }

        Ok(())
    }
}

impl<S> Service<Request<Body>> for Ratelimit<S>
where
    S: Clone + Service<Request<Body>, Response = AxumResponse> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let ip = match get_ip(&req) {
            Some(ip) => ip,
            None => {
                return Box::pin(async move {
                    Ok(Response(
                        StatusCode::BAD_REQUEST,
                        Error::MalformedIp {
                            message: "Could not resolve an IP address from the request. \
                                We require a valid IP address to protect us from DoS attacks.",
                        },
                    )
                    .promote(req.headers())
                    .into_response())
                });
            }
        };

        match self.handle_ratelimit(req.headers(), ip) {
            Ok(_) => {
                let clone = self.inner.clone();
                let mut inner = std::mem::replace(&mut self.inner, clone);
                let (rate, per) = (self.rate, self.per);
                let count = self.buckets.get(&ip).map_or(rate, |b| b.value().count);

                Box::pin(async move {
                    let mut result = inner.call(req).await?;
                    let headers = result.headers_mut();

                    Self::insert_headers(rate, per, headers);
                    headers.insert("X-RateLimit-Remaining", count.to_string().parse().unwrap());

                    Ok(result)
                })
            }
            Err(res) => Box::pin(async { Ok(res) }),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RatelimitLayer(pub u16, pub u16);

impl<S> Layer<S> for RatelimitLayer {
    type Service = Ratelimit<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Ratelimit::new(inner, self.0, self.1)
    }
}

// Implmentation from https://github.com/imbolc/axum-client-ip/blob/main/src/lib.rs
fn get_ip(req: &Request<Body>) -> Option<IpAddr> {
    let headers = req.headers();

    headers
        .get("x-forwarded-for")
        .and_then(|hv| hv.to_str().ok())
        .and_then(|s| s.split(',').find_map(|s| s.trim().parse::<IpAddr>().ok()))
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|hv| hv.to_str().ok())
                .and_then(|s| s.parse::<IpAddr>().ok())
        })
        .or_else(|| {
            headers.get_all(FORWARDED).iter().find_map(|hv| {
                hv.to_str()
                    .ok()
                    .and_then(|s| ForwardedHeaderValue::from_forwarded(s).ok())
                    .and_then(|f| {
                        f.iter()
                            .filter_map(|fs| fs.forwarded_for.as_ref())
                            .find_map(|ident| match ident {
                                Identifier::SocketAddr(a) => Some(a.ip()),
                                Identifier::IpAddr(ip) => Some(*ip),
                                _ => None,
                            })
                    })
            })
        })
        .or_else(|| {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(addr)| addr.ip())
        })
}

macro_rules! ratelimit {
    ($rate:expr, $per:expr) => {{
        tower::ServiceBuilder::new()
            .layer(axum::error_handling::HandleErrorLayer::new(|_| async {
                unreachable!()
            }))
            .layer(tower::buffer::BufferLayer::new(1024))
            .layer(crate::ratelimit::RatelimitLayer($rate, $per))
    }};
}

pub(crate) use ratelimit;
