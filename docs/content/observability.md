# Observability

Scotty includes a comprehensive observability stack for monitoring application health, performance, and behavior. The stack provides metrics, distributed tracing, and visualization through industry-standard tools.

## Architecture

```
Scotty Application
       ↓ (OTLP over gRPC)
OpenTelemetry Collector (port 4317)
       ├─→ Jaeger (distributed traces)
       └─→ VictoriaMetrics (metrics storage)
              ↓
           Grafana (visualization & dashboards)
```

### Components

- **OpenTelemetry Collector**: Receives telemetry data from Scotty via OTLP protocol and routes it to appropriate backends
- **VictoriaMetrics**: High-performance time-series database for metrics storage (30-day retention)
- **Jaeger**: Distributed tracing backend for request traces and spans
- **Grafana**: Visualization platform with pre-configured dashboards

### Resource Usage

The observability stack requires approximately:
- **Memory**: 180-250 MB total
- **CPU**: Minimal (< 5% on modern systems)
- **Disk**: ~1-2 GB for 30 days of metrics retention

## Prometheus Compatibility & Flexibility

**All metrics are fully Prometheus-compatible.** The stack uses open standards (OTLP, PromQL, W3C Trace Context) and components are interchangeable.

### Metric Format

- OpenTelemetry format (`scotty.metric.name`) → Prometheus format (`scotty_metric_name_total`)
- Standard types: Counter, Gauge, Histogram, UpDownCounter
- Attributes become labels (`method`, `status`, `path`)

### Replace Components as Needed

**Use Prometheus instead of VictoriaMetrics:**
Update `otel-collector-config.yaml` exporter from `prometheusremotewrite` to `prometheus` endpoint, then swap VictoriaMetrics for Prometheus in docker-compose.

**Alternative backends:** Thanos, Cortex, M3DB, InfluxDB, Datadog, New Relic, Honeycomb, Grafana Cloud

**Alternative visualization:** Prometheus UI, VictoriaMetrics vmui, Chronograf, commercial dashboards

**Alternative tracing:** Zipkin, Tempo, Elasticsearch + Jaeger, Lightstep, Honeycomb

**Multi-backend export example:**
```yaml
# otel-collector-config.yaml - export to multiple destinations
service:
  pipelines:
    metrics:
      exporters: [prometheusremotewrite/victoria, prometheusremotewrite/thanos, otlp/datadog]
```

### Integration Patterns

**Remote write to existing Prometheus:**
```yaml
exporters:
  prometheusremotewrite:
    endpoint: "https://your-prometheus.company.com/api/v1/write"
```

**Federation from VictoriaMetrics:**
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'scotty'
    metrics_path: '/api/v1/export/prometheus'
    params:
      match[]: ['{__name__=~"scotty_.*"}']
    static_configs:
      - targets: ['victoriametrics:8428']
```

**Service discovery:** Standard Kubernetes/Consul Prometheus SD works with VictoriaMetrics API.

### Why VictoriaMetrics Default

Chosen for development convenience: lower memory usage, single binary, Prometheus-compatible, free. Swap for Prometheus in production if preferred.

## Quick Start

### Prerequisites

The observability stack requires Traefik for .ddev.site domain routing. Start Traefik first:

```bash
cd apps/traefik
docker-compose up -d
```

### Starting the Observability Stack

```bash
cd observability
docker-compose up -d
```

This will start all four services:
- OpenTelemetry Collector
- VictoriaMetrics
- Jaeger
- Grafana

### Enabling Metrics in Scotty

Configure Scotty to export telemetry data using the `SCOTTY__TELEMETRY` environment variable:

**Enable both metrics and traces:**
```bash
SCOTTY__TELEMETRY=metrics,traces cargo run --bin scotty
```

**Enable only metrics:**
```bash
SCOTTY__TELEMETRY=metrics cargo run --bin scotty
```

**Production deployment** (in docker-compose.yml or .env):
```yaml
environment:
  - SCOTTY__TELEMETRY=metrics,traces
```

### Accessing Services

Once running, access the services at:

| Service | URL | Credentials |
|---------|-----|-------------|
| Grafana | http://grafana.ddev.site | admin/admin |
| Jaeger UI | http://jaeger.ddev.site | (none) |
| VictoriaMetrics | http://vm.ddev.site | (none) |

## Available Metrics

Scotty exports comprehensive metrics covering all major subsystems. All metrics use the `scotty.` prefix.

### Log Streaming Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `scotty_log_streams_active` | Gauge | Number of active log streams |
| `scotty_log_streams_total` | Counter | Total log streams created |
| `scotty_log_stream_duration_seconds` | Histogram | Duration of log streaming sessions |
| `scotty_log_stream_lines_total` | Counter | Total log lines streamed to clients |
| `scotty_log_stream_errors_total` | Counter | Log streaming errors |

**Use Cases:**
- Monitor concurrent log stream load
- Detect log streaming errors
- Analyze log stream duration patterns

### Shell Session Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `scotty_shell_sessions_active` | Gauge | Number of active shell sessions |
| `scotty_shell_sessions_total` | Counter | Total shell sessions created |
| `scotty_shell_session_duration_seconds` | Histogram | Shell session duration |
| `scotty_shell_session_errors_total` | Counter | Shell session errors |
| `scotty_shell_session_timeouts_total` | Counter | Sessions ended due to timeout |

**Use Cases:**
- Monitor active shell connections
- Track session timeout rates
- Identify shell session errors

### WebSocket Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `scotty_websocket_connections_active` | Gauge | Active WebSocket connections |
| `scotty_websocket_messages_sent_total` | Counter | Messages sent to clients |
| `scotty_websocket_messages_received_total` | Counter | Messages received from clients |
| `scotty_websocket_auth_failures_total` | Counter | WebSocket authentication failures |

**Use Cases:**
- Monitor real-time connection count
- Track message throughput
- Detect authentication issues

### Task Output Streaming Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `scotty_tasks_active` | Gauge | Active task output streams |
| `scotty_tasks_total` | Counter | Total tasks executed |
| `scotty_task_duration_seconds` | Histogram | Task execution duration |
| `scotty_task_failures_total` | Counter | Failed tasks |
| `scotty_task_output_lines_total` | Counter | Task output lines streamed |

**Use Cases:**
- Monitor task execution load
- Track task failure rates
- Analyze output streaming performance

### HTTP Server Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `scotty_http_requests_active` | UpDownCounter | Currently processing requests |
| `scotty_http_requests_total` | Counter | Total HTTP requests |
| `scotty_http_request_duration_seconds` | Histogram | Request processing time |

**Attributes:**
- `method`: HTTP method (GET, POST, etc.)
- `path`: Request path
- `status`: HTTP status code

**Use Cases:**
- Monitor API endpoint performance
- Track request rates by endpoint
- Identify slow requests

### Memory Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `scotty_memory_rss_bytes` | Gauge | Resident Set Size (RSS) in bytes |
| `scotty_memory_virtual_bytes` | Gauge | Virtual memory size in bytes |

**Use Cases:**
- Monitor memory consumption
- Detect memory leaks
- Capacity planning

### Application Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `scotty_apps_total` | Gauge | Total managed applications |
| `scotty_apps_by_status` | Gauge | Apps grouped by status |
| `scotty_app_services_count` | Histogram | Services per application distribution |
| `scotty_app_last_check_age_seconds` | Histogram | Time since last health check |

**Attributes:**
- `status`: Application status (running, stopped, etc.)

**Use Cases:**
- Monitor application fleet size
- Track application health check timeliness
- Analyze service distribution

### Tokio Runtime Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `scotty_tokio_workers_count` | Gauge | Number of Tokio worker threads |
| `scotty_tokio_tasks_active` | Gauge | Active instrumented tasks |
| `scotty_tokio_tasks_dropped_total` | Counter | Completed/dropped tasks |
| `scotty_tokio_poll_count_total` | Counter | Total task polls |
| `scotty_tokio_poll_duration_seconds` | Histogram | Task poll duration |
| `scotty_tokio_slow_poll_count_total` | Counter | Slow task polls (>1ms) |
| `scotty_tokio_idle_duration_seconds` | Histogram | Task idle time between polls |
| `scotty_tokio_scheduled_count_total` | Counter | Task scheduling events |
| `scotty_tokio_first_poll_delay_seconds` | Histogram | Delay from creation to first poll |

**Use Cases:**
- Monitor async runtime health
- Detect slow tasks blocking the runtime
- Optimize task scheduling

## Grafana Dashboard

Scotty includes a pre-configured Grafana dashboard (`scotty-metrics.json`) that visualizes all available metrics.

### Dashboard Sections

1. **Log Streaming**: Active streams, throughput, duration percentiles, errors
2. **Shell Sessions**: Active sessions, creation rate, duration, errors & timeouts
3. **WebSocket & Tasks**: Connection metrics, message rates, task execution
4. **Memory Usage**: RSS and virtual memory trends
5. **HTTP Server**: Request rates, active requests, latencies
6. **Tokio Runtime**: Worker threads, task lifecycle, poll metrics
7. **Application Metrics**: App count, status distribution, health checks

### Accessing the Dashboard

1. Open Grafana: http://grafana.ddev.site
2. Login with `admin` / `admin` (change on first login)
3. Navigate to **Dashboards** → **Scotty Metrics**

The dashboard auto-refreshes every 5 seconds and shows data from the last hour by default.

## PromQL Query Examples

### Request Rate by HTTP Status

```promql
sum by (status) (rate(scotty_http_requests_total[5m]))
```

### P95 Request Latency

```promql
histogram_quantile(0.95, rate(scotty_http_request_duration_seconds_bucket[5m]))
```

### WebSocket Connection Churn

```promql
rate(scotty_websocket_connections_total[5m])
```

### Memory Growth Rate

```promql
deriv(scotty_memory_rss_bytes[10m])
```

### Active Resources Summary

```promql
# All active resources
scotty_log_streams_active +
scotty_shell_sessions_active +
scotty_websocket_connections_active +
scotty_tasks_active
```

## Distributed Tracing

When traces are enabled (`SCOTTY__TELEMETRY=traces` or `metrics,traces`), Scotty exports distributed traces to Jaeger.

### Viewing Traces

1. Open Jaeger UI: http://jaeger.ddev.site
2. Select **scotty** service
3. Search for traces by operation or timeframe

### Key Operations

- `HTTP POST /apps/create`: Application creation
- `HTTP GET /apps/info/{name}`: Application info retrieval
- `log_stream_handler`: Log streaming operations
- `shell_session_handler`: Shell session management

Traces include timing information, error status, and contextual metadata for debugging request flows.

## Troubleshooting

### No Metrics Appearing in Grafana

1. **Check Scotty is exporting metrics:**
   ```bash
   # Verify SCOTTY__TELEMETRY is set
   echo $SCOTTY__TELEMETRY

   # Should be 'metrics' or 'metrics,traces'
   ```

2. **Verify OpenTelemetry Collector is receiving data:**
   ```bash
   docker logs otel-collector
   # Look for: "Trace received"
   ```

3. **Check VictoriaMetrics has data:**
   ```bash
   curl http://vm.ddev.site/api/v1/label/__name__/values | jq
   # Should list scotty_* metrics
   ```

4. **Restart the stack:**
   ```bash
   cd observability
   docker-compose restart
   ```

### High Memory Usage

If VictoriaMetrics uses too much memory, adjust retention:

```yaml
# observability/docker-compose.yml
services:
  victoriametrics:
    command:
      - '-retentionPeriod=14d'  # Reduce from 30d
```

### Connection Refused Errors

Ensure Traefik is running:
```bash
docker ps | grep traefik
cd apps/traefik
docker-compose up -d
```

### Grafana Dashboard Not Loading

1. Check dashboard file exists: `observability/grafana/dashboards/scotty-metrics.json`
2. Restart Grafana: `docker-compose restart grafana`
3. Check Grafana logs: `docker logs grafana`

## Configuration

### OpenTelemetry Collector

Configuration file: `observability/otel-collector-config.yaml`

Key settings:
- **OTLP Receiver**: Port 4317 (gRPC)
- **Exporters**: Jaeger (traces), Prometheus Remote Write (metrics to VictoriaMetrics)
- **Batch Processor**: Batches telemetry for efficiency

### VictoriaMetrics

Configuration via docker-compose environment:
- **Retention**: 30 days (`-retentionPeriod=30d`)
- **Storage path**: `/victoria-metrics-data`
- **HTTP port**: 8428

### Grafana

Configuration in `observability/grafana/provisioning/`:
- **Datasources**: VictoriaMetrics (Prometheus type)
- **Dashboards**: Auto-provisioned from `dashboards/` directory

## Production Recommendations

### Resource Allocation

For production deployments, allocate resources based on scale:

**Small deployment** (< 10 apps):
- VictoriaMetrics: 256 MB memory
- OpenTelemetry Collector: 128 MB memory
- Grafana: 256 MB memory

**Medium deployment** (10-50 apps):
- VictoriaMetrics: 512 MB memory
- OpenTelemetry Collector: 256 MB memory
- Grafana: 512 MB memory

**Large deployment** (50+ apps):
- VictoriaMetrics: 1 GB+ memory
- OpenTelemetry Collector: 512 MB memory
- Grafana: 512 MB memory

### Alerting

Configure Grafana alerts for critical metrics:

- **High error rate**: `rate(scotty_http_requests_total{status="500"}[5m]) > 0.1`
- **Memory leak**: `deriv(scotty_memory_rss_bytes[30m]) > 1000000`
- **High WebSocket failures**: `rate(scotty_websocket_auth_failures_total[5m]) > 1`
- **Task failures**: `rate(scotty_task_failures_total[5m]) > 0.5`

### Data Retention

Adjust retention based on compliance and capacity:

```yaml
# observability/docker-compose.yml
services:
  victoriametrics:
    command:
      - '-retentionPeriod=90d'  # 3 months for compliance
```

### Security

**Production checklist:**
- [ ] Change Grafana default password
- [ ] Enable Grafana authentication (OAuth, LDAP, etc.)
- [ ] Use TLS for Grafana access
- [ ] Restrict Jaeger UI access
- [ ] Firewall VictoriaMetrics port (8428)
- [ ] Use secure networks for OTLP traffic

## Further Reading

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [VictoriaMetrics Documentation](https://docs.victoriametrics.com/)
- [Grafana Documentation](https://grafana.com/docs/)
- [PromQL Tutorial](https://prometheus.io/docs/prometheus/latest/querying/basics/)
