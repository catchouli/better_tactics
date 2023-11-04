FROM rust:1.67 as builder
WORKDIR /usr/src/better-tactics
COPY . .

# Install node/npm using nvm.
ENV NODE_VERSION=21.1.0
RUN apt-get install -y curl
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
ENV PATH="/root/.nvm/versions/node/v${NODE_VERSION}/bin:${PATH}"

# Run cargo build.
RUN cargo install --locked --path .

# Now copy the build output to a leaner image.
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/better-tactics /usr/local/bin/better-tactics
CMD ["better-tactics"]
