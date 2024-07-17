FROM rust:1.79-slim-bookworm as chef
RUN apt-get update -y && \
    apt-get install --no-install-recommends -y pkg-config make g++ libssl-dev curl jq && \
    rustup target add x86_64-unknown-linux-gnu && \
    apt-get clean && rm -rf /var/lib/apt/lists/*


# Install cargo-chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin yafbds --bin yafbdsctl

FROM node:18 as frontend-builder
WORKDIR /app
COPY ./frontend /app
RUN yarn install && yarn build


FROM debian:bookworm-slim
ARG APP=/app

RUN apt-get update \
    && apt-get --no-install-recommends install -y curl ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

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

COPY --from=builder /app/target/release/yafbds ${APP}/yafbds
COPY --from=builder /app/target/release/yafbdsctl ${APP}/yafbdsctl
COPY --from=builder /app/config ${APP}/config
COPY --from=frontend-builder /app/public ${APP}/frontend/
RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

HEALTHCHECK --interval=30s --timeout=3s \
    CMD curl -f http://localhost:21342/api/health || exit 1

ENV RUST_LOG=api
CMD ["./yafbds"]
