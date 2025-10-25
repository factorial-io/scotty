# Scotty Observability Stack

This directory contains the observability infrastructure for Scotty, providing metrics, traces, and visualization.

## Components

- **Jaeger**: Distributed tracing backend (http://jaeger.ddev.site)
- **OpenTelemetry Collector**: Unified telemetry pipeline
- **VictoriaMetrics**: Time-series metrics database (http://vm.ddev.site)
- **Grafana**: Visualization and dashboards (http://grafana.ddev.site)

## Quick Start

Start the observability stack:

```bash
cd observability
docker-compose up -d
```

Stop the stack:

```bash
docker-compose down
```

## Accessing Services

**Note**: The .ddev.site URLs require Traefik to be running. Start Traefik first:

```bash
cd apps/traefik
docker-compose up -d
```

Then access:

- **Grafana**: http://grafana.ddev.site (admin/admin)
- **Jaeger UI**: http://jaeger.ddev.site
- **VictoriaMetrics**: http://vm.ddev.site

## Configuration

- `docker-compose.yml`: Service definitions for the observability stack
- `otel-collector-config.yaml`: OpenTelemetry Collector configuration
- `grafana/provisioning/`: Auto-provisioned datasources
- `grafana/dashboards/`: Grafana dashboard definitions

## Enabling Metrics in Scotty

Set the telemetry environment variable:

```bash
SCOTTY__TELEMETRY=metrics,traces cargo run --bin scotty
```

Or just metrics:

```bash
SCOTTY__TELEMETRY=metrics cargo run --bin scotty
```

## Architecture

```
Scotty Application
       ↓ (OTLP)
OpenTelemetry Collector
       ├─→ Jaeger (traces)
       └─→ VictoriaMetrics (metrics)
              ↓
           Grafana (visualization)
```

## Data Retention

- VictoriaMetrics: 30 days (configurable in docker-compose.yml)
- Jaeger: In-memory (data lost on restart)
