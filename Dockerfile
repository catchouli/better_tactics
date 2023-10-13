FROM rust:1.67 as builder
WORKDIR /usr/src/better-tactics
COPY . .
RUN cargo install --locked --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/better-tactics /usr/local/bin/better-tactics
CMD ["better-tactics"]
