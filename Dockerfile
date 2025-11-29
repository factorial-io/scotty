# Base Rust environment with cargo-chef
FROM rust:1.91-slim-bookworm AS chef
RUN apt-get update -y && \
    apt-get install --no-install-recommends -y pkg-config make g++ libssl-dev curl jq && \
    rustup target add x86_64-unknown-linux-gnu && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Install cargo-chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# TypeScript generation stage using cargo-chef
FROM chef AS ts-generator
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json --package scotty-ts-generator
COPY . .
RUN cargo run --package scotty-ts-generator

# Frontend build stage
FROM oven/bun:1 AS frontend-builder
WORKDIR /app
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile
# Install all potential Rollup platform binaries as optional dependencies
# This ensures the build works across different target platforms
RUN bun add \
    @rollup/rollup-linux-x64-gnu \
    @rollup/rollup-linux-arm64-gnu \
    @rollup/rollup-linux-x64-musl \
    @rollup/rollup-linux-arm64-musl \
    --optional
COPY frontend/ ./
COPY --from=ts-generator /app/frontend/src/generated ./src/generated
RUN bun run build

# Main application build stage
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
COPY --from=frontend-builder /app/build /app/frontend/build
RUN cargo build --release -p scotty -p scottyctl

FROM debian:bookworm-slim
ARG APP=/app

RUN apt-get update \
    && apt-get --no-install-recommends install -y curl ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

# install docker
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN curl -fsSL https://get.docker.com | bash

# Set the Docker Compose version to install
ENV DOCKER_COMPOSE_VERSION=2.29.0
# Download and install Docker Compose
RUN curl -L "https://github.com/docker/compose/releases/download/v${DOCKER_COMPOSE_VERSION}/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose && \
    chmod +x /usr/local/bin/docker-compose

EXPOSE 21342

ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /app/target/release/scotty ${APP}/scotty
COPY --from=builder /app/target/release/scottyctl ${APP}/scottyctl

# Only copy non-sensitive config files (model and blueprints)
# DO NOT copy default.yaml or policy.yaml - these should be mounted at runtime
# to avoid baking secrets or deployment-specific configs into the image
COPY --from=builder /app/config/casbin/model.conf ${APP}/config/casbin/model.conf
COPY --from=builder /app/config/blueprints ${APP}/config/blueprints

# We don't need to copy the frontend files separately anymore since they're embedded in the binary
# RUN chown -R $APP_USER:$APP_USER ${APP}
# USER $APP_USER
WORKDIR ${APP}

HEALTHCHECK --interval=10s --timeout=2s --start-period=15s --retries=3 \
    CMD curl -f http://localhost:21342/api/v1/health || exit 1

ENV RUST_LOG=api
CMD ["./scotty"]
