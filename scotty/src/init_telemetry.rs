use anyhow::Result;
use tracing::info;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
use tracing::warn;
use tracing_subscriber::{layer::SubscriberExt, Layer};
use tracing_subscriber::{registry, EnvFilter};

#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
use opentelemetry::trace::TracerProvider;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
use opentelemetry_sdk::trace::{TraceError, Tracer};
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
use tracing::Subscriber;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
use tracing_opentelemetry::OpenTelemetryLayer;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
use tracing_subscriber::registry::LookupSpan;

#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub fn build_otel_layer<S>() -> Result<OpenTelemetryLayer<S, Tracer>, TraceError>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    use init_tracing_opentelemetry::{
        init_propagator, //stdio,
        otlp::traces::{identity, init_tracerprovider},
        resource::DetectResource,
    };
    use opentelemetry::global;
    let otel_rsrc = DetectResource::default()
        .with_fallback_service_name(env!("CARGO_PKG_NAME"))
        .with_fallback_service_version(env!("CARGO_PKG_VERSION"))
        .build();
    let tracerprovider =
        init_tracerprovider(otel_rsrc, identity).map_err(|e| TraceError::Other(Box::new(e)))?;
    // to not send trace somewhere, but continue to create and propagate,...
    // then send them to `axum_tracing_opentelemetry::stdio::WriteNoWhere::default()`
    // or to `std::io::stdout()` to print
    //
    // let otel_tracer = stdio::init_tracer(
    //     otel_rsrc,
    //     stdio::identity::<stdio::WriteNoWhere>,
    //     stdio::WriteNoWhere::default(),
    // )?;
    init_propagator()?;
    let layer = tracing_opentelemetry::layer()
        .with_error_records_to_exceptions(true)
        .with_tracer(tracerprovider.tracer(""));
    global::set_tracer_provider(tracerprovider);
    Ok(layer)
}

pub fn build_reduced_logger_text<S>() -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    if cfg!(debug_assertions) {
        Box::new(
            tracing_subscriber::fmt::layer()
                .with_line_number(false)
                .with_thread_names(false)
                .with_timer(tracing_subscriber::fmt::time::SystemTime)
                .with_target(true)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE) // Disable span list display
                .event_format(
                    tracing_subscriber::fmt::format().compact(), // Use compact format
                ),
        )
    } else {
        Box::new(
            tracing_subscriber::fmt::layer()
                //.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_timer(tracing_subscriber::fmt::time::SystemTime)
                .with_target(true),
        )
    }
}

pub fn build_loglevel_filter_layer() -> tracing_subscriber::filter::EnvFilter {
    // filter what is output on log (fmt)
    // std::env::set_var("RUST_LOG", "warn,otel::tracing=info,otel=debug");
    std::env::set_var(
        "RUST_LOG",
        format!(
            // `otel::tracing` should be a level info to emit opentelemetry trace & span
            // `otel::setup` set to debug to log detected resources, configuration read and infered
            // Filter out verbose HTTP request details from axum tracing
            "{},otel::tracing=trace,otel=debug,axum_tracing_opentelemetry=error",
            std::env::var("RUST_LOG")
                .or_else(|_| std::env::var("OTEL_LOG_LEVEL"))
                .unwrap_or_else(|_| "warn".to_string())
        ),
    );
    EnvFilter::from_default_env()
}

// Full telemetry implementation (with OpenTelemetry)
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub fn init_telemetry_and_tracing(settings: &Option<String>) -> Result<()> {
    //setup a temporary subscriber to log output during setup
    use init_tracing_opentelemetry::config::TracingConfig;
    let subscriber = registry()
        .with(build_loglevel_filter_layer())
        .with(TracingConfig::default().build_layer()?);
    let _guard = tracing::subscriber::set_default(subscriber);
    info!("init logging & tracing");

    let mut tracing_enabled = false;
    let mut metrics_enabled = false;

    if let Some(settings) = settings {
        let splitted = settings.to_lowercase();
        let splitted: Vec<&str> = splitted.split(',').collect();

        tracing_enabled = splitted.contains(&"traces");
        metrics_enabled = splitted.contains(&"metrics");
    }

    let tracing_result = match tracing_enabled {
        true => {
            let subscriber = tracing_subscriber::registry()
                .with(build_otel_layer()?)
                .with(build_loglevel_filter_layer())
                .with(build_reduced_logger_text());
            tracing::subscriber::set_global_default(subscriber)?;

            Ok(())
        }
        false => {
            let subscriber = registry()
                .with(build_loglevel_filter_layer())
                .with(build_reduced_logger_text());
            tracing::subscriber::set_global_default(subscriber)?;

            Ok(())
        }
    };

    // Always initialize metrics when telemetry features are compiled
    // If not enabled via config, the recorder is still created but metrics won't be sent
    // This prevents panics when code calls metrics().record_*() unconditionally
    match crate::metrics::init_metrics() {
        Ok(_) => {
            if metrics_enabled {
                info!("OpenTelemetry metrics initialized and enabled");
            } else {
                info!("OpenTelemetry metrics initialized (sending disabled via config)");
            }
        }
        Err(e) => warn!("Failed to initialize metrics: {}", e),
    }

    tracing_result
}

// Minimal telemetry implementation (no OpenTelemetry)
#[cfg(not(any(feature = "telemetry-grpc", feature = "telemetry-http")))]
pub fn init_telemetry_and_tracing(_settings: &Option<String>) -> Result<()> {
    info!("init logging (no-telemetry mode)");

    let subscriber = registry()
        .with(build_loglevel_filter_layer())
        .with(build_reduced_logger_text());
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Basic tracing initialized (OpenTelemetry disabled)");
    Ok(())
}
