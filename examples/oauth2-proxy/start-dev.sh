#!/bin/bash

# Scotty OAuth Development Helper Script
# This script helps you start Scotty in different authentication modes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

show_help() {
    cat << EOF
Scotty OAuth Development Helper

USAGE:
    $0 <mode> [options]

MODES:
    dev      - Development mode (no authentication required)
    oauth    - OAuth mode (requires GitLab OAuth setup)
    bearer   - Bearer token mode (traditional authentication)  
    full     - All modes running simultaneously

OPTIONS:
    --build     - Rebuild containers before starting
    --logs      - Follow logs after starting
    --help      - Show this help message

EXAMPLES:
    # Start in development mode (fastest for development)
    $0 dev

    # Start OAuth mode (requires .env file with GitLab credentials)
    $0 oauth --build

    # Start bearer token mode
    $0 bearer --logs

    # Start all modes for comparison
    $0 full

ENVIRONMENT SETUP:
    For OAuth mode, create .env file with:
    - GITLAB_CLIENT_ID=your-client-id
    - GITLAB_CLIENT_SECRET=your-client-secret
    - COOKIE_SECRET=random-32-char-string

    For bearer mode:
    - SCOTTY_ACCESS_TOKEN=your-api-token

URLS:
    - Dev mode:    http://localhost:3000
    - OAuth mode:  http://localhost (redirects to GitLab)
    - Bearer mode: http://localhost:3001
    - Traefik:     http://localhost:8080
EOF
}

build_flag=""
follow_logs=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        dev|oauth|bearer|full)
            mode="$1"
            shift
            ;;
        --build)
            build_flag="--build"
            shift
            ;;
        --logs)
            follow_logs=true
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

# Check environment setup
check_oauth_env() {
    # Check if environment variables are available (from 1Password or .env file)
    if [ -n "$GITLAB_CLIENT_ID" ] && [ -n "$GITLAB_CLIENT_SECRET" ] && [ -n "$COOKIE_SECRET" ]; then
        echo "✅ OAuth environment variables found"
        return 0
    fi
    
    # Try loading from .env file as fallback
    if [ -f .env ]; then
        echo "📄 Loading environment from .env file..."
        source .env
        
        if [ -n "$GITLAB_CLIENT_ID" ] && [ -n "$GITLAB_CLIENT_SECRET" ] && [ -n "$COOKIE_SECRET" ]; then
            echo "✅ OAuth environment variables loaded from .env"
            return 0
        fi
    fi
    
    echo "❌ Missing required OAuth environment variables"
    echo ""
    echo "Required variables:"
    echo "  - GITLAB_CLIENT_ID"
    echo "  - GITLAB_CLIENT_SECRET" 
    echo "  - COOKIE_SECRET"
    echo ""
    echo "To use 1Password:"
    echo "  op run --env-file=\"./.env.1password\" -- $0 oauth"
    echo ""
    echo "Or create a .env file with the required variables"
    return 1
}

check_bearer_env() {
    if [ -z "$SCOTTY_ACCESS_TOKEN" ]; then
        echo "Using default bearer token: demo-token-12345"
        export SCOTTY_ACCESS_TOKEN="demo-token-12345"
    fi
}

echo "🚀 Starting Scotty in '$mode' mode..."

case $mode in
    dev)
        echo "📍 Development mode - no authentication required"
        echo "🔗 Access at: http://localhost:3000"
        docker-compose -f docker-compose.dev.yml --profile dev up $build_flag -d
        ;;
    oauth)
        if check_oauth_env; then
            echo "🔐 OAuth mode - authentication via GitLab"
            echo "🔗 Access at: http://localhost (will redirect to GitLab)"
            echo "📊 Traefik dashboard: http://localhost:8080"
            
            docker-compose -f docker-compose.dev.yml --profile oauth up $build_flag -d
        else
            exit 1
        fi
        ;;
    bearer)
        check_bearer_env
        echo "🔑 Bearer token mode - traditional API token authentication"
        echo "🔗 Access at: http://localhost:3001"
        echo "🔐 Token: $SCOTTY_ACCESS_TOKEN"
        docker-compose -f docker-compose.dev.yml --profile bearer up $build_flag -d
        ;;
    full)
        if ! check_oauth_env; then
            echo "Skipping OAuth components due to missing configuration"
            echo "Starting dev and bearer modes only..."
            check_bearer_env
            docker-compose -f docker-compose.dev.yml --profile dev --profile bearer up $build_flag -d
        else
            check_bearer_env
            echo "🚀 Full stack - all authentication modes"
            echo "🔗 Dev mode:    http://localhost:3000"
            echo "🔗 OAuth mode:  http://localhost"
            echo "🔗 Bearer mode: http://localhost:3001"
            echo "📊 Traefik:     http://localhost:8080"
            docker-compose -f docker-compose.dev.yml --profile full up $build_flag -d
        fi
        ;;
esac

if [ "$follow_logs" = true ]; then
    echo ""
    echo "📋 Following logs (Ctrl+C to stop)..."
    docker-compose -f docker-compose.dev.yml logs -f
else
    echo ""
    echo "✅ Services started successfully!"
    echo "📋 View logs with: $0 $mode --logs"
    echo "🛑 Stop services with: docker-compose -f docker-compose.dev.yml down"
fi