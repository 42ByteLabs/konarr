# Build the Agent 
FROM docker.io/library/rust:1.83-alpine as agent

WORKDIR /app

COPY . .

RUN apk add --no-cache pkgconf alpine-sdk openssl-dev perl musl-dev && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo build -p konarr-cli -F agent --release --target x86_64-unknown-linux-musl

FROM docker.io/library/rust:1.83-alpine as tools

# Download all the tools install scripts
# TODO: Add SHA256 checks
RUN apk add --no-cache curl && \
    curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh -o /usr/local/bin/install-syft && \
    curl -sSfL https://raw.githubusercontent.com/anchore/grype/main/install.sh -o /usr/local/bin/install-grype && \
    curl -sSfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh -o /usr/local/bin/install-trivy
# Install Syft by default
RUN sh /usr/local/bin/install-syft -b /usr/local/toolcache

# Final image
FROM docker.io/library/alpine:3.21

ENV KONARR_CONTAINER=true
ENV KONARR_AGENT_MONITORING=true
ENV KONARR_AGENT_AUTO_CREATE=true
ENV KONARR_AUTO_INSTALL=false

ENV CONFIG=/config/konarr.yml

WORKDIR /app

COPY --from=tools /usr/local/bin/* /usr/local/bin
COPY --from=tools /usr/local/toolcache/ /usr/local/toolcache
COPY --from=agent /app/target/x86_64-unknown-linux-musl/release/konarr-cli /usr/local/bin/konarr-cli

ENTRYPOINT ["konarr-cli"]
