//! Middleware for recording rate limit metrics

use axum::http::{Request, Response, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Layer that records rate limit metrics
#[derive(Clone)]
pub struct RateLimitMetricsLayer {
    tier_name: &'static str,
}

impl RateLimitMetricsLayer {
    pub fn new(tier_name: &'static str) -> Self {
        Self { tier_name }
    }
}

impl<S> Layer<S> for RateLimitMetricsLayer {
    type Service = RateLimitMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMetricsService {
            inner,
            tier_name: self.tier_name,
        }
    }
}

/// Service that records rate limit metrics
#[derive(Clone)]
pub struct RateLimitMetricsService<S> {
    inner: S,
    tier_name: &'static str,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for RateLimitMetricsService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let mut inner = self.inner.clone();
        let tier_name = self.tier_name;

        Box::pin(async move {
            let response = inner.call(req).await?;

            // Record metric if rate limited
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                super::metrics::record_rate_limit_hit(tier_name);
            }

            Ok(response)
        })
    }
}
