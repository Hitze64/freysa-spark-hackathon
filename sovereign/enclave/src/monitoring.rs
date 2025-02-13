use futures::Future;
use prometheus::{HistogramOpts, HistogramVec, Registry};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tonic::codegen::http::{request, response};
use tonic::Code;
use tower::{Layer, Service};

pub struct Metrics {
    pub registry: Registry,
    pub grpc_request_duration_seconds: HistogramVec,
    pub stream_request_duration_seconds: HistogramVec,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        let buckets = vec![0.001, 0.01, 0.1, 1.0];
        let grpc_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new("grpc_request_duration_seconds", "gRPC request duration in seconds")
                .buckets(buckets.clone()),
            &["service", "method", "code"],
        )
        .expect("metric can be created");
        let stream_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new("stream_request_duration_seconds", "request duration in seconds")
                .buckets(buckets),
            &["protocol", "method", "code"],
        )
        .expect("metric can be created");
        registry
            .register(Box::new(grpc_request_duration_seconds.clone()))
            .expect("collector can be registered");
        registry
            .register(Box::new(stream_request_duration_seconds.clone()))
            .expect("collector can be registered");
        Self { registry, grpc_request_duration_seconds, stream_request_duration_seconds }
    }
}

fn parse_grpc_path(path: &str) -> (String, String) {
    match path.chars().next() {
        Some('/') => {
            if let Some(second_slash) = path[1..].find('/') {
                // Note that second_slash is relative to the slice path[1..]
                let service = &path[1..second_slash + 1];
                let method = &path[second_slash + 2..];
                (service.to_string(), method.to_string())
            } else {
                (String::new(), path.to_string())
            }
        }
        _ => (String::new(), path.to_string()),
    }
}

#[derive(Clone)]
pub struct MetricsInterceptor<S> {
    metrics: Arc<Metrics>,
    service: S,
}

impl<S> MetricsInterceptor<S> {
    pub fn new(metrics: Arc<Metrics>, service: S) -> Self {
        Self { metrics, service }
    }
}

impl<S, B, C> Service<request::Request<B>> for MetricsInterceptor<S>
where
    S: Service<request::Request<B>, Response = response::Response<C>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = MetricsFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: request::Request<B>) -> Self::Future {
        let path = req.uri().path().to_owned();
        let f = self.service.call(req);

        MetricsFuture::new(self.metrics.clone(), path, f)
    }
}

#[pin_project::pin_project]
pub struct MetricsFuture<F> {
    metrics: Arc<Metrics>,
    path: String,
    started_at: Option<Instant>,
    #[pin]
    inner: F,
}

impl<F> MetricsFuture<F> {
    pub fn new(metrics: Arc<Metrics>, path: String, inner: F) -> Self {
        Self { metrics, path, started_at: None, inner }
    }
}

impl<F, B, E> Future for MetricsFuture<F>
where
    F: Future<Output = Result<response::Response<B>, E>>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let (service, method) = parse_grpc_path(this.path);

        let started_at = this.started_at.get_or_insert_with(Instant::now);

        if let Poll::Ready(result) = this.inner.poll(cx) {
            let elapsed = started_at.elapsed().as_secs_f64();

            let code = match &result {
                Ok(response) => response
                    .headers()
                    .get("grpc-status")
                    .and_then(|s| s.to_str().ok())
                    .and_then(|s| s.parse::<i32>().ok())
                    .map(Code::from)
                    .unwrap_or(Code::Ok),
                Err(_) => Code::Unknown,
            };

            let code_str = format!("{:?}", code);

            this.metrics
                .grpc_request_duration_seconds
                .with_label_values(&[&service, &method, &code_str])
                .observe(elapsed);

            Poll::Ready(result)
        } else {
            Poll::Pending
        }
    }
}

#[derive(Clone)]
pub struct MetricsLayer {
    pub metrics: std::sync::Arc<Metrics>,
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsInterceptor<S>;

    fn layer(&self, service: S) -> Self::Service {
        MetricsInterceptor::new(self.metrics.clone(), service)
    }
}
