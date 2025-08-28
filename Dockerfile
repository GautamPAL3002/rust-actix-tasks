# Multi-stage build
FROM rust:1.79 as builder
WORKDIR /app
COPY Cargo.toml .
RUN mkdir src && echo "fn main() {println!(\"placeholder\");}" > src/main.rs
RUN cargo build --release
COPY . .
RUN cargo build --release

# Runtime
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libsqlite3-0 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rust-actix-tasks /app/rust-actix-tasks
COPY migrations /app/migrations
ENV DATABASE_URL=sqlite:///app/data.db
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
CMD ["/app/rust-actix-tasks"]
