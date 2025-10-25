use anyhow::Result;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_logger_text;
use opentelemetry::trace::{TraceError, TracerProvider};
use opentelemetry_sdk::trace::Tracer;
use tracing::{info, warn, Subscriber};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, registry::LookupSpan, Layer};
use tracing_subscriber::{registry, EnvFilter};

pub fn build_otel_layer<S>() -> Result<OpenTelemetryLayer<S, Tracer>, TraceError>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    use init_tracing_opentelemetry::{
        init_propagator, //stdio,
        otlp,
        resource::DetectResource,
    };
    use opentelemetry::global;
    let otel_rsrc = DetectResource::default()
        .with_fallback_service_name(env!("CARGO_PKG_NAME"))
        .with_fallback_service_version(env!("CARGO_PKG_VERSION"))
        .build();
    let tracerprovider = otlp::init_tracerprovider(otel_rsrc, otlp::identity)?;
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
    S: Subscriber + for<'a> LookupSpan<'a>,
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

pub fn init_telemetry_and_tracing(settings: &Option<String>) -> Result<()> {
    //setup a temporary subscriber to log output during setup
    let subscriber = registry()
        .with(build_loglevel_filter_layer())
        .with(build_logger_text());
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

    if metrics_enabled {
        match crate::metrics::init_metrics() {
            Ok(_) => {
                info!("OpenTelemetry metrics initialized successfully");
            }
            Err(e) => warn!("Failed to initialize metrics: {}", e),
        }
    }

    tracing_result
}
