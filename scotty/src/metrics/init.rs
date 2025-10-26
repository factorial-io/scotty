use super::instruments::ScottyMetrics;
use anyhow::Result;
use opentelemetry::metrics::MeterProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{MetricExporter, WithExportConfig};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    Resource,
};
use std::time::Duration;

/// Initialize OpenTelemetry metrics with OTLP exporter
///
/// Sets up the global MeterProvider to export metrics via OTLP
/// to the OpenTelemetry Collector.
pub fn init_metrics() -> Result<ScottyMetrics> {
    // Get OTLP endpoint from environment or use default
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://otel-collector:4317".to_string());

    // Build OTLP metric exporter
    let exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    // Periodic reader (export every 10s)
    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(10))
        .build();

    // Resource with service info
    let resource = Resource::builder()
        .with_service_name(env!("CARGO_PKG_NAME"))
        .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
        .build();

    // Build and set global provider
    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();

    global::set_meter_provider(provider.clone());

    // Create metrics
    let meter = provider.meter(env!("CARGO_PKG_NAME"));
    let metrics = ScottyMetrics::new(meter);

    // Set global metrics instance
    super::set_metrics(metrics.clone());

    Ok(metrics)
}
