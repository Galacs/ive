# FROM rust:1.66.0
# WORKDIR /usr/src/ive
# COPY . .
# RUN apt update -y
# RUN apt install libavutil-dev libavformat-dev libavfilter-dev clang libavcodec-dev libavformat-dev libavutil-dev pkg-config libavdevice-dev -y
# RUN cargo install --path .
# CMD ["ive"]

# FROM rust:1.66.0 AS chef 
# # We only pay the installation cost once, 
# # it will be cached from the second build onwards
# RUN cargo install cargo-chef 
# WORKDIR app

# FROM chef AS planner
# COPY . .
# RUN cargo chef prepare  --recipe-path recipe.json

# FROM chef AS builder
# RUN apt update && apt install libavutil-dev libavformat-dev libavfilter-dev clang libavcodec-dev libavformat-dev libavutil-dev pkg-config libavdevice-dev -y
# COPY --from=planner /app/recipe.json recipe.json
# # Build dependencies - this is the caching Docker layer!
# RUN cargo chef cook --release --recipe-path recipe.json
# # Build application
# COPY . .
# RUN apt update && apt install libavutil-dev libavformat-dev libavfilter-dev clang libavcodec-dev libavformat-dev libavutil-dev pkg-config libavdevice-dev -y
# RUN cargo build --release --bin ive

# # We do not need the Rust toolchain to run the binary!
# FROM debian:buster-slim AS runtime
# RUN apt update && apt install libavutil-dev libavformat-dev libavfilter-dev clang libavcodec-dev libavformat-dev libavutil-dev pkg-config libavdevice-dev -y
# WORKDIR app
# COPY --from=builder /app/target/release/ive /usr/local/bin
# ENTRYPOINT ["/usr/local/bin/ive"]


FROM rust:1.66.0 AS chef 
# We only pay the installation cost once, 
# it will be cached from the second build onwards
RUN cargo install cargo-chef 
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt update && apt install libavutil-dev libavformat-dev libavfilter-dev clang libavcodec-dev libavformat-dev libavutil-dev pkg-config libavdevice-dev -y
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release -p ive --bin ive
RUN cargo build --release -p worker --bin worker

# We do not need the Rust toolchain to run the binary!
FROM debian:bullseye-slim AS ive_runtime
WORKDIR app
# RUN apt update && apt install  -y
COPY --from=builder /app/target/release/ive /usr/local/bin
ENTRYPOINT ["/usr/local/bin/ive"]


FROM debian:bullseye-slim AS worker_runtime
WORKDIR app
RUN apt-get update && apt-get install ffmpeg -y
COPY --from=builder /app/target/release/worker /usr/local/bin
ENTRYPOINT ["/usr/local/bin/worker"]