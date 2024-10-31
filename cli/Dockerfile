# Build the Agent 
FROM docker.io/library/rust:1.80-alpine as agent

WORKDIR /app

COPY . .

RUN apk add --no-cache pkgconf alpine-sdk openssl-dev perl musl-dev curl && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo build -p konarr-cli -F agent --release --target x86_64-unknown-linux-musl
# Install Syft
RUN curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh -s -- -b /usr/local/bin

FROM docker.io/library/alpine:3.20

ENV KONARR_CONTAINER=true
ENV KONARR_AGENT_MONITORING=true
ENV KONARR_AGENT_AUTO_CREATE=true

ENV CONFIG=/config/konarr.yml

WORKDIR /app

COPY --from=agent /app/target/x86_64-unknown-linux-musl/release/konarr-cli /usr/local/bin/konarr-cli
COPY --from=agent /usr/local/bin/syft /usr/local/bin/syft

ENTRYPOINT ["konarr-cli"]
