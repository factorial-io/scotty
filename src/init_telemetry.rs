use anyhow::Result;
use init_tracing_opentelemetry::resource::DetectResource;
use init_tracing_opentelemetry::tracing_subscriber_ext::build_logger_text;
use init_tracing_opentelemetry::{init_propagator, otlp};
use opentelemetry::trace::TraceError;
use opentelemetry_sdk::trace::Tracer;
use tracing::{info, warn, Subscriber};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, registry::LookupSpan, Layer};
use tracing_subscriber::{registry, EnvFilter};

fn build_otel_layer<S>() -> std::result::Result<OpenTelemetryLayer<S, Tracer>, TraceError>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let otel_rsrc = DetectResource::default()
        .with_fallback_service_name(env!("CARGO_PKG_NAME"))
        .with_fallback_service_version(env!("CARGO_PKG_VERSION"))
        .build();
    let otel_tracer = otlp::init_tracer(otel_rsrc, otlp::identity)?;
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
    Ok(tracing_opentelemetry::layer()
        .with_error_records_to_exceptions(true)
        .with_tracer(otel_tracer))
}

pub fn build_reduced_logger_text<S>() -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    if cfg!(debug_assertions) {
        Box::new(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_line_number(true)
                .with_thread_names(true)
                .with_timer(tracing_subscriber::fmt::time::uptime()),
        )
    } else {
        Box::new(
            tracing_subscriber::fmt::layer()
                .json()
                //.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_timer(tracing_subscriber::fmt::time::uptime()),
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
            "{},otel::tracing=trace,otel=debug",
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

    info!(
        "Tracing enabled: {}, metrics enabled: {}",
        tracing_enabled, metrics_enabled
    );

    let tracing_result = match tracing_enabled {
        true => {
            let otel_layer = build_otel_layer()?;

            let subscriber = registry()
                .with(otel_layer)
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
        // @todo: Handle metrics
        warn!("Metrics are not yet implemented");
    }

    tracing_result
}
