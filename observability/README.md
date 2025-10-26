# Scotty Observability Stack

This directory contains the observability infrastructure for Scotty, providing comprehensive metrics, distributed tracing, and visualization capabilities.

## Components

- **OpenTelemetry Collector**: Telemetry data pipeline (receives OTLP, routes to backends)
- **VictoriaMetrics**: High-performance time-series metrics database
- **Jaeger**: Distributed tracing backend for request traces
- **Grafana**: Visualization platform with pre-configured Scotty dashboard

## Quick Start

### Prerequisites

Start Traefik for .ddev.site domain routing:

```bash
cd apps/traefik
docker-compose up -d
```

### Start the Observability Stack

```bash
cd observability
docker-compose up -d
```

### Enable Telemetry in Scotty

Set the telemetry environment variable when running Scotty:

```bash
# Enable both metrics and traces
SCOTTY__TELEMETRY=metrics,traces cargo run --bin scotty

# Or just metrics
SCOTTY__TELEMETRY=metrics cargo run --bin scotty
```

### Access the Services

| Service | URL | Credentials |
|---------|-----|-------------|
| **Grafana** | http://grafana.ddev.site | admin/admin |
| **Jaeger UI** | http://jaeger.ddev.site | (none) |
| **VictoriaMetrics** | http://vm.ddev.site | (none) |

## What Gets Monitored

Scotty exports 40+ metrics covering:

- **Log Streaming**: Active streams, throughput, duration, errors
- **Shell Sessions**: Active sessions, creation rate, timeouts, errors
- **WebSocket**: Connections, message rates, authentication failures
- **Task Execution**: Output streams, line counts, failures
- **HTTP Server**: Request rates, latencies, active requests (by endpoint)
- **Memory**: RSS and virtual memory usage
- **Applications**: App count, status distribution, health checks
- **Tokio Runtime**: Worker threads, task polling, scheduling metrics

## Pre-configured Dashboard

The Grafana dashboard (`grafana/dashboards/scotty-metrics.json`) provides:

1. **Log Streaming** section with real-time stream metrics
2. **Shell Sessions** with active connections and timeouts
3. **WebSocket & Tasks** showing connection and execution metrics
4. **Memory Usage** tracking resource consumption
5. **HTTP Server** performance by endpoint
6. **Tokio Runtime** async runtime health
7. **Application Metrics** for managed apps

Access it at: http://grafana.ddev.site → Dashboards → Scotty Metrics

## Architecture

```
Scotty Application
       ↓ (OTLP/gRPC on port 4317)
OpenTelemetry Collector
       ├─→ Jaeger (traces)
       └─→ VictoriaMetrics (metrics)
              ↓
           Grafana (visualization)
```

## Configuration Files

- `docker-compose.yml`: Service definitions and resource limits
- `otel-collector-config.yaml`: OpenTelemetry Collector pipeline configuration
- `grafana/provisioning/datasources/`: Auto-configured VictoriaMetrics datasource
- `grafana/dashboards/`: Pre-built Scotty metrics dashboard

## Data Retention

- **VictoriaMetrics**: 30 days (configurable via `-retentionPeriod` in docker-compose.yml)
- **Jaeger**: In-memory only (data lost on restart)

To adjust retention:

```yaml
# docker-compose.yml
services:
  victoriametrics:
    command:
      - '-retentionPeriod=14d'  # Reduce to 14 days
```

## Resource Usage

Expected resource consumption:

- **Total Memory**: ~180-250 MB
- **CPU**: < 5% on modern systems
- **Disk**: 1-2 GB for 30 days of metrics

## Common Operations

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker logs otel-collector
docker logs victoriametrics
docker logs grafana
```

### Restart Services

```bash
docker-compose restart
```

### Stop and Clean Up

```bash
# Stop services (keeps data)
docker-compose stop

# Stop and remove containers (keeps volumes)
docker-compose down

# Complete cleanup (removes data!)
docker-compose down -v
```

### Check Metrics Collection

Verify metrics are being collected:

```bash
# List all metrics in VictoriaMetrics
curl http://vm.ddev.site/api/v1/label/__name__/values | jq

# Check specific metric
curl "http://vm.ddev.site/api/v1/query?query=scotty_http_requests_total" | jq
```

## Troubleshooting

### No Data in Grafana

1. Verify Scotty has telemetry enabled: `echo $SCOTTY__TELEMETRY`
2. Check OpenTelemetry Collector logs: `docker logs otel-collector`
3. Verify VictoriaMetrics has data: `curl http://vm.ddev.site/api/v1/label/__name__/values`
4. Restart the stack: `docker-compose restart`

### .ddev.site URLs Not Working

Ensure Traefik is running:

```bash
docker ps | grep traefik

# If not running:
cd apps/traefik
docker-compose up -d
```

### High Memory Usage

Reduce VictoriaMetrics retention period in `docker-compose.yml`:

```yaml
services:
  victoriametrics:
    command:
      - '-retentionPeriod=7d'  # Reduce from 30d
```

## Production Recommendations

For production deployments:

1. **Change Grafana password**: Default is `admin/admin`
2. **Enable authentication**: Configure Grafana OAuth or LDAP
3. **Set up alerts**: Use Grafana alerting for critical metrics
4. **Secure access**: Use firewall rules or VPN for observability services
5. **Adjust retention**: Balance disk usage with compliance requirements
6. **Resource limits**: Set appropriate memory/CPU limits in docker-compose

## Further Documentation

For detailed information on metrics, queries, and advanced configuration, see:

📖 **[Complete Observability Guide](../docs/content/observability.md)**

Includes:
- Complete metrics reference with descriptions
- PromQL query examples
- Production deployment guide
- Alerting recommendations
- Security best practices
