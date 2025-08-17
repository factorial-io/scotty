# Scotty Development Setup

Simple development setup with no authentication required. Perfect for local development and testing.

## Quick Start

```bash
# Start Scotty in development mode
docker-compose up -d

# Access Scotty directly
open http://localhost:3000
```

## What This Provides

- **Direct access** to Scotty on port 3000
- **No authentication** required - automatic dev user login
- **Full Scotty functionality** for creating and managing apps
- **Docker integration** for managing containerized applications

## Development User

- **Email**: developer@localhost  
- **Name**: Local Developer
- **Access**: Full admin access to all Scotty features

## Configuration

The setup uses:
- `config/development.yaml` for Scotty configuration
- `apps/` directory for deployed applications  
- Direct Docker socket access for container management

## Next Steps

Once you're ready to test OAuth authentication, use the `oauth2-proxy-oauth` setup instead.