# Build the Agent 
FROM docker.io/library/rust:1.80-alpine as agent

WORKDIR /app

COPY . .

RUN apk add --no-cache pkgconf alpine-sdk openssl-dev perl musl-dev && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo build -p konarr-cli --release --target x86_64-unknown-linux-musl

FROM docker.io/library/alpine:3.20

WORKDIR /app

COPY --from=agent /app/target/x86_64-unknown-linux-musl/release/konarr-cli /usr/local/bin/konarr-cli

ENTRYPOINT ["konarr-cli"]
