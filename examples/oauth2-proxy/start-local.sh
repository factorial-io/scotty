#!/bin/bash

# Scotty Local Development Helper Script
# This script starts Scotty locally (no Docker) in different auth modes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

show_help() {
    cat << EOF
Scotty Local Development Helper (Host-based)

USAGE:
    $0 <mode> [options]

MODES:
    dev      - Development mode (no authentication required)
    oauth    - OAuth mode (requires oauth2-proxy running)
    bearer   - Bearer token mode (traditional authentication)

OPTIONS:
    --build     - Build Scotty before starting
    --logs      - Run with verbose logging
    --help      - Show this help message

EXAMPLES:
    # Start in development mode (fastest for development)
    $0 dev

    # Build and start in development mode
    $0 dev --build

    # Start OAuth mode with oauth2-proxy
    $0 oauth --logs

ENVIRONMENT SETUP:
    For OAuth mode, you'll need oauth2-proxy running.
    Use docker-compose to start just the proxy components:
    
    docker-compose -f docker-compose.dev.yml --profile oauth up oauth2-proxy traefik

URLS:
    - Dev/Bearer mode: http://localhost:3000
    - OAuth mode:      http://localhost (via Traefik)
    - Frontend dev:    http://localhost:5173 (if running 'npm run dev')

NOTES:
    - Scotty will be compiled and run locally on your host
    - Docker is only used for oauth2-proxy and Traefik in OAuth mode
    - Much faster iteration than full Docker setup
EOF
}

build_flag=false
verbose_logs=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        dev|oauth|bearer)
            mode="$1"
            shift
            ;;
        --build)
            build_flag=true
            shift
            ;;
        --logs)
            verbose_logs=true
            shift
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

if [ -z "$mode" ]; then
    echo "Error: No mode specified"
    show_help
    exit 1
fi

# Build if requested
if [ "$build_flag" = true ]; then
    echo "🔨 Building Scotty..."
    cd "$PROJECT_ROOT"
    cargo build --bin scotty
fi

# Set up environment variables based on mode
setup_environment() {
    export SCOTTY__API__BIND_ADDRESS="0.0.0.0:3000"
    
    case $mode in
        dev)
            export SCOTTY__API__AUTH_MODE="dev"
            export SCOTTY__API__DEV_USER_EMAIL="developer@localhost"
            export SCOTTY__API__DEV_USER_NAME="Local Developer"
            ;;
        oauth)
            export SCOTTY__API__AUTH_MODE="oauth"
            export SCOTTY__API__OAUTH_REDIRECT_URL="/oauth2/start"
            ;;
        bearer)
            export SCOTTY__API__AUTH_MODE="bearer"
            if [ -z "$SCOTTY__API__ACCESS_TOKEN" ]; then
                export SCOTTY__API__ACCESS_TOKEN="demo-token-12345"
                echo "Using default bearer token: demo-token-12345"
            fi
            ;;
    esac
    
    if [ "$verbose_logs" = true ]; then
        export RUST_LOG="scotty=debug,scottyctl=debug"
    else
        export RUST_LOG="scotty=info"
    fi
}

# Check OAuth prerequisites
check_oauth_setup() {
    if [ "$mode" = "oauth" ]; then
        echo "🔍 Checking OAuth prerequisites..."
        
        # Check if Traefik is running
        if ! curl -s http://localhost:8080/api/version > /dev/null 2>&1; then
            echo "❌ Traefik not running on port 8080"
            echo ""
            echo "For OAuth mode, start the proxy components first:"
            echo "  cd $SCRIPT_DIR"
            echo "  docker-compose -f docker-compose.dev.yml --profile oauth up -d traefik oauth2-proxy"
            echo ""
            echo "Or start all proxy components:"
            echo "  ./start-dev.sh oauth  # This uses Docker for Scotty too"
            exit 1
        fi
        
        echo "✅ OAuth infrastructure appears to be running"
    fi
}

echo "🚀 Starting Scotty locally in '$mode' mode..."

setup_environment
check_oauth_setup

cd "$PROJECT_ROOT"

case $mode in
    dev)
        echo "📍 Development mode - no authentication required"
        echo "🔗 Direct access: http://localhost:3000"
        echo "🔗 With frontend: http://localhost:5173 (run 'cd frontend && npm run dev')"
        ;;
    oauth)
        echo "🔐 OAuth mode - authentication via GitLab"
        echo "🔗 Access via Traefik: http://localhost"
        echo "🔗 Direct access: http://localhost:3000 (will show auth errors)"
        ;;
    bearer)
        echo "🔑 Bearer token mode - traditional API token authentication"
        echo "🔗 Direct access: http://localhost:3000"
        echo "🔐 Token: $SCOTTY__API__ACCESS_TOKEN"
        ;;
esac

echo "📋 Environment variables set:"
env | grep "SCOTTY__" | sort

echo ""
echo "🎯 Starting Scotty..."

# Start Scotty
./target/debug/scotty