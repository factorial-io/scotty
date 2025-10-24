# OpenTelemetry Metrics Backend Research

## Context

Scotty already uses OpenTelemetry for distributed tracing with:
- ✅ `opentelemetry` 0.28.0 with trace features
- ✅ `opentelemetry-otlp` for OTLP protocol
- ✅ Jaeger all-in-one running in docker-compose.yml (port 4318 OTLP receiver)
- ✅ `tracing-opentelemetry` integration

**Goal**: Extend OpenTelemetry usage to include metrics while:
- Using OpenTelemetry metrics API (not Prometheus directly)
- Keeping resource requirements low (no Prometheus)
- Integrating with existing docker-compose.yml and Jaeger setup
- Visualizing with Grafana

## Architecture Options

### Current Setup
```
Scotty (Rust)
  └─> OTLP/gRPC (traces) ─> Jaeger (port 4318)
```

### Target Architecture
```
Scotty (Rust)
  ├─> OTLP/gRPC (traces) ─────────────┐
  └─> OTLP/gRPC (metrics) ────────────┤
                                      ▼
                          OpenTelemetry Collector
                                ├─> Jaeger (traces)
                                └─> Metrics Backend ─> Grafana
```

## Metrics to Track

### Unified Output System Metrics

```rust
// Using opentelemetry::metrics API
use opentelemetry::metrics::{Counter, Gauge, Histogram};

// Log streaming
log_streams_active: Gauge<u64>
log_streams_total: Counter<u64>
log_stream_duration: Histogram<f64>
log_lines_received: Counter<u64>
log_stream_errors: Counter<u64>
log_stream_bytes: Counter<u64>

// Shell sessions
shell_sessions_active: Gauge<u64>
shell_sessions_total: Counter<u64>
shell_session_duration: Histogram<f64>
shell_session_errors: Counter<u64>
shell_session_timeouts: Counter<u64>

// WebSocket
websocket_connections_active: Gauge<u64>
websocket_connections_total: Counter<u64>
websocket_messages_sent: Counter<u64>
websocket_messages_received: Counter<u64>
websocket_auth_failures: Counter<u64>

// Tasks
tasks_active: Gauge<u64>
tasks_total: Counter<u64>
task_duration: Histogram<f64>
task_output_lines: Counter<u64>
task_output_buffer_bytes: Gauge<u64>

// System health
memory_usage_bytes: Gauge<u64>
cpu_usage_percent: Gauge<f64>
uptime_seconds: Counter<u64>
```

## Lightweight OTLP-Compatible Options

### Option 1: OpenTelemetry Collector + VictoriaMetrics ⭐ RECOMMENDED

**Architecture:**
```
Scotty
  └─> OTLP ─> OTel Collector ┬─> Jaeger (traces)
                              └─> VictoriaMetrics (metrics, Prometheus format)
                                   └─> Grafana
```

**Pros:**
- OTel Collector is lightweight (~30MB memory)
- VictoriaMetrics very efficient (~50-100MB)
- Single unified pipeline for traces + metrics
- Grafana works with Prometheus datasource
- Can extend to logs later (full observability stack)
- Flexible exporters (can swap backends easily)

**Cons:**
- Two components instead of one
- Extra hop for metrics

**Total Resource Requirements:**
- OTel Collector: ~30-50 MB
- VictoriaMetrics: ~50-100 MB
- **Total: ~100-150 MB** (still lighter than Prometheus alone)

**docker-compose.yml addition:**
```yaml
services:
  otel-collector:
    image: otel/opentelemetry-collector:0.95.0
    command: ["--config=/etc/otel-collector-config.yaml"]
    volumes:
      - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml
    ports:
      - "4317:4317"  # OTLP gRPC receiver
      - "4318:4318"  # OTLP HTTP receiver
      - "8888:8888"  # Metrics endpoint
    depends_on:
      - victoriametrics

  victoriametrics:
    image: victoriametrics/victoria-metrics:latest
    ports:
      - "8428:8428"
    volumes:
      - vm-data:/victoria-metrics-data
    command:
      - "--storageDataPath=/victoria-metrics-data"
      - "--retentionPeriod=30d"

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana-provisioning:/etc/grafana/provisioning
    depends_on:
      - victoriametrics
      - jaeger

volumes:
  vm-data:
  grafana-data:
```

### Option 2: Grafana Alloy (formerly Grafana Agent)

**Architecture:**
```
Scotty
  └─> OTLP ─> Grafana Alloy ┬─> Jaeger (traces)
                             └─> Grafana Cloud / Mimir (metrics)
```

**Pros:**
- Single lightweight agent
- Native OTLP support
- Grafana-native solution
- Can output to multiple backends

**Cons:**
- Requires Grafana Cloud or Mimir for metrics
- Mimir is resource-heavy for local dev
- Less flexible than OTel Collector

**Not recommended** - Too Grafana-ecosystem specific

### Option 3: VictoriaMetrics with Native OTLP

**Architecture:**
```
Scotty
  ├─> OTLP ─> Jaeger (traces, port 4318)
  └─> OTLP ─> VictoriaMetrics (metrics, different port)
```

**Pros:**
- Minimal components (reuse existing Jaeger container)
- VictoriaMetrics v1.93+ has experimental OTLP support
- Simple architecture

**Cons:**
- OTLP support in VictoriaMetrics is experimental
- Still need separate endpoints for traces vs metrics
- Less flexibility

**Resource Requirements:**
- VictoriaMetrics: ~50-100 MB
- **Total: ~50-100 MB** (lightest option)

**docker-compose.yml addition:**
```yaml
services:
  victoriametrics:
    image: victoriametrics/victoria-metrics:latest
    ports:
      - "8428:8428"  # HTTP API
      - "4318:4318"  # OTLP receiver (conflicts with Jaeger!)
    volumes:
      - vm-data:/victoria-metrics-data
    command:
      - "--storageDataPath=/victoria-metrics-data"
      - "--retentionPeriod=30d"
      - "--opentelemetry.http.listenAddr=:4318"

volumes:
  vm-data:
```

**Issue:** Port 4318 conflict with Jaeger!

### Option 4: SigNoz (All-in-One Observability)

**Architecture:**
```
Scotty
  └─> OTLP ─> SigNoz (traces + metrics + logs)
               └─> Built-in UI
```

**Pros:**
- All-in-one solution (traces, metrics, logs)
- Native OTLP support
- Built-in dashboards
- ClickHouse backend (efficient)

**Cons:**
- Heavy (~1GB memory for full stack)
- Complex deployment (multiple containers)
- Overkill for just metrics
- Not using Grafana

**Not recommended** - Too heavy, doesn't use Grafana

### Option 5: Jaeger + Prometheus Exporter

**Architecture:**
```
Scotty
  └─> OTLP ─> Jaeger (with metrics) ─> Prometheus Remote Write ─> VictoriaMetrics
```

**Pros:**
- Reuse existing Jaeger container
- Jaeger can expose metrics

**Cons:**
- Jaeger is primarily for traces, metrics support limited
- Complex configuration
- Not the intended use case

**Not recommended** - Wrong tool for the job

## Detailed Comparison

| Feature | OTel Collector + VM | VM Native OTLP | Grafana Alloy | SigNoz |
|---------|---------------------|----------------|---------------|--------|
| **Memory (MB)** | 100-150 | 50-100 | 80-120 | 1000+ |
| **Components** | 2 + Jaeger | 1 + Jaeger | 1 + Backend | Many |
| **OTLP Maturity** | ✅ Stable | 🟡 Experimental | ✅ Stable | ✅ Stable |
| **Port Conflict** | ❌ No | ⚠️ Yes (4318) | ❌ No | ❌ No |
| **Grafana Ready** | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No |
| **Flexibility** | ✅ High | 🟡 Medium | 🟡 Medium | ❌ Low |
| **Setup Complexity** | Medium | Low | Medium | High |
| **Production Ready** | ✅ Yes | 🟡 Experimental | ✅ Yes | ✅ Yes |

## Recommendation: OpenTelemetry Collector + VictoriaMetrics

### Why This Combination?

1. **Unified Pipeline**: Single OTLP endpoint for both traces and metrics
2. **Proven Stack**: Both components are production-ready and widely used
3. **Lightweight**: Total ~100-150MB (much less than Prometheus)
4. **Flexible**: Easy to swap backends or add exporters
5. **Future-Proof**: Can add logs to same pipeline later
6. **No Port Conflicts**: Clean separation of concerns

### Implementation Plan

#### Step 1: Add OpenTelemetry Metrics Dependencies

```toml
# Cargo.toml - workspace dependencies
[workspace.dependencies]
opentelemetry = { version = "0.28.0", features = ["trace", "metrics"] }
opentelemetry_sdk = { version = "0.28", features = [
    "trace",
    "metrics",
    "rt-tokio",
] }
opentelemetry-otlp = { version = "0.28.0", features = [
    "trace",
    "metrics",
    "http-proto",
    "reqwest-client",
] }
```

#### Step 2: Create OTel Collector Configuration

```yaml
# otel-collector-config.yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 10s
    send_batch_size: 1024

exporters:
  # Traces to Jaeger
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true

  # Metrics to VictoriaMetrics (Prometheus Remote Write)
  prometheusremotewrite:
    endpoint: http://victoriametrics:8428/api/v1/write
    tls:
      insecure: true

  # Debug exporter for testing
  logging:
    loglevel: debug

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [jaeger, logging]

    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [prometheusremotewrite, logging]
```

#### Step 3: Update docker-compose.yml

```yaml
services:
  # Existing Jaeger service
  jaeger:
    image: jaegertracing/all-in-one:${JAEGER_VERSION:-latest}
    ports:
      - "16686:16686"  # Jaeger UI
      - "14250:14250"  # Model proto
    environment:
      - LOG_LEVEL=debug
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.jaeger.rule=Host(`jaeger.ddev.site`)"
      - "traefik.http.services.jaeger.loadbalancer.server.port=16686"

  # NEW: OpenTelemetry Collector
  otel-collector:
    image: otel/opentelemetry-collector:0.95.0
    command: ["--config=/etc/otel-collector-config.yaml"]
    volumes:
      - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml
    ports:
      - "4317:4317"  # OTLP gRPC
      - "4318:4318"  # OTLP HTTP
      - "8888:8888"  # Prometheus metrics about the collector itself
    depends_on:
      - jaeger
      - victoriametrics
    labels:
      - "traefik.enable=false"

  # NEW: VictoriaMetrics
  victoriametrics:
    image: victoriametrics/victoria-metrics:latest
    ports:
      - "8428:8428"
    volumes:
      - vm-data:/victoria-metrics-data
    command:
      - "--storageDataPath=/victoria-metrics-data"
      - "--retentionPeriod=30d"
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.victoriametrics.rule=Host(`vm.ddev.site`)"
      - "traefik.http.services.victoriametrics.loadbalancer.server.port=8428"

  # NEW: Grafana
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning:ro
      - ./grafana/dashboards:/var/lib/grafana/dashboards:ro
    depends_on:
      - victoriametrics
      - jaeger
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.grafana.rule=Host(`grafana.ddev.site`)"
      - "traefik.http.services.grafana.loadbalancer.server.port=3000"

volumes:
  vm-data:
  grafana-data:
```

#### Step 4: Create Metrics Module in Scotty

```rust
// scotty/src/metrics/mod.rs
use opentelemetry::{
    metrics::{Counter, Gauge, Histogram, Meter, MeterProvider},
    KeyValue,
};
use opentelemetry_sdk::metrics::SdkMeterProvider;

pub struct ScottyMetrics {
    // Log streaming
    pub log_streams_active: Gauge<u64>,
    pub log_streams_total: Counter<u64>,
    pub log_stream_duration: Histogram<f64>,

    // Shell sessions
    pub shell_sessions_active: Gauge<u64>,
    pub shell_sessions_total: Counter<u64>,

    // WebSocket
    pub websocket_connections: Gauge<u64>,
    pub websocket_messages_sent: Counter<u64>,

    // Tasks
    pub tasks_active: Gauge<u64>,
    pub tasks_total: Counter<u64>,
}

impl ScottyMetrics {
    pub fn new(meter: Meter) -> Self {
        Self {
            log_streams_active: meter
                .u64_gauge("scotty.log_streams.active")
                .with_description("Current number of active log streams")
                .init(),

            log_streams_total: meter
                .u64_counter("scotty.log_streams.total")
                .with_description("Total log streams created")
                .init(),

            log_stream_duration: meter
                .f64_histogram("scotty.log_stream.duration")
                .with_description("Log stream duration in seconds")
                .with_unit("s")
                .init(),

            shell_sessions_active: meter
                .u64_gauge("scotty.shell_sessions.active")
                .with_description("Current number of active shell sessions")
                .init(),

            shell_sessions_total: meter
                .u64_counter("scotty.shell_sessions.total")
                .with_description("Total shell sessions created")
                .init(),

            websocket_connections: meter
                .u64_gauge("scotty.websocket.connections")
                .with_description("Current WebSocket connections")
                .init(),

            websocket_messages_sent: meter
                .u64_counter("scotty.websocket.messages_sent")
                .with_description("Total WebSocket messages sent")
                .init(),

            tasks_active: meter
                .u64_gauge("scotty.tasks.active")
                .with_description("Current number of active tasks")
                .init(),

            tasks_total: meter
                .u64_counter("scotty.tasks.total")
                .with_description("Total tasks executed")
                .init(),
        }
    }
}

// Initialize in main.rs
pub fn init_metrics() -> Result<ScottyMetrics, Box<dyn std::error::Error>> {
    let meter_provider = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://otel-collector:4317"),
        )
        .build()?;

    let meter = meter_provider.meter("scotty");
    Ok(ScottyMetrics::new(meter))
}
```

#### Step 5: Instrument Services

```rust
// In LogStreamingService
impl LogStreamingService {
    pub async fn start_stream(&self, ...) -> Result<...> {
        // Increment counters
        self.metrics.log_streams_total.add(1, &[]);
        self.metrics.log_streams_active.add(1, &[]);

        let start_time = std::time::Instant::now();

        // ... existing stream logic ...

        // On completion
        self.metrics.log_streams_active.add(-1, &[]);
        self.metrics.log_stream_duration.record(
            start_time.elapsed().as_secs_f64(),
            &[]
        );
    }
}
```

#### Step 6: Grafana Datasource Provisioning

```yaml
# grafana/provisioning/datasources/datasources.yaml
apiVersion: 1

datasources:
  - name: VictoriaMetrics
    type: prometheus
    access: proxy
    url: http://victoriametrics:8428
    isDefault: true
    jsonData:
      timeInterval: 15s

  - name: Jaeger
    type: jaeger
    access: proxy
    url: http://jaeger:16686
    jsonData:
      tracesToLogs:
        datasourceUid: 'loki'
```

#### Step 7: Create Grafana Dashboard

```json
// grafana/dashboards/scotty-metrics.json
{
  "dashboard": {
    "title": "Scotty - Unified Output System",
    "panels": [
      {
        "title": "Active Log Streams",
        "targets": [
          {
            "expr": "scotty_log_streams_active"
          }
        ]
      },
      {
        "title": "Active Shell Sessions",
        "targets": [
          {
            "expr": "scotty_shell_sessions_active"
          }
        ]
      },
      {
        "title": "WebSocket Connections",
        "targets": [
          {
            "expr": "scotty_websocket_connections"
          }
        ]
      }
    ]
  }
}
```

## Resource Requirements Summary

| Component | Memory | CPU | Storage |
|-----------|--------|-----|---------|
| OTel Collector | 30-50 MB | < 2% | Minimal |
| VictoriaMetrics | 50-100 MB | < 5% | ~100MB/30 days |
| Grafana | 100-150 MB | < 5% | ~50MB |
| Jaeger (existing) | 200-300 MB | < 5% | Varies |
| **Total New** | **180-250 MB** | **< 10%** | **~150MB** |

## Timeline Estimate

- **Day 1**: Add OTel metrics dependencies and create metrics module
- **Day 2**: Instrument log streaming and shell services
- **Day 3**: Add OTel Collector and VictoriaMetrics to docker-compose
- **Day 4**: Set up Grafana with dashboards
- **Day 5**: Testing, documentation, and refinement
- **Total**: ~5 days

## Success Criteria

- [x] Uses OpenTelemetry metrics API (native integration)
- [x] Total memory overhead < 250 MB
- [x] Integrates with existing Jaeger in docker-compose.yml
- [x] No port conflicts
- [x] Grafana dashboards showing real-time metrics
- [x] 30-day retention configured
- [x] Simple docker-compose up experience

## Open Questions

1. Should we use gRPC (port 4317) or HTTP (port 4318) for OTLP?
   - **Recommendation**: gRPC (4317) - better performance, smaller payloads

2. Should metrics be always-on or configurable?
   - **Recommendation**: Always on, but allow disabling via env var

3. What scrape interval for OTel Collector?
   - **Recommendation**: 15s (balance between freshness and overhead)

4. Should we include system metrics (CPU, memory) from the host?
   - **Recommendation**: Yes, but via separate receiver in OTel Collector

5. Bundle observability stack in separate compose file or main one?
   - **Recommendation**: Separate `docker-compose.observability.yml` for opt-in

## Next Steps

1. ✅ Update beads issue scotty-5 with this research
2. Create proof-of-concept branch
3. Add OTel metrics dependencies
4. Create basic metrics module
5. Test with OTel Collector + VictoriaMetrics locally
6. Benchmark memory usage
7. Create initial Grafana dashboard
8. Document setup in README
