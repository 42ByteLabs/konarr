# This Docker Compose file builds both the Konarr server and frontent 
# but generate a final image to be used.

# Build the Server
FROM docker.io/library/rust:1.82-alpine as server 

WORKDIR /app

COPY . .

RUN apk add --no-cache pkgconf alpine-sdk openssl-dev perl musl-dev curl git && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo build -p konarr-server --release --target x86_64-unknown-linux-musl

FROM docker.io/library/rust:1.82-alpine as tools

# Download all the tools install scripts
RUN apk add --no-cache curl && \
    curl -sSfL https://raw.githubusercontent.com/anchore/grype/main/install.sh -o /usr/local/bin/install-grype

# Install Syft by default
RUN sh /usr/local/bin/install-grype -b /usr/local/toolcache

# Build the Web
FROM docker.io/library/node:22.12-alpine as web

WORKDIR /app

COPY . .

RUN cd frontend && \
    npm install && \
    npm run build || true

# Final Image
FROM docker.io/library/alpine:3.20

EXPOSE 9000
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=9000
# Make sure the server can find the client
ENV KONARR_DATA_PATH=/data
ENV KONARR_DB_PATH=/data/konarr.db
ENV KONARR_SERVER_FRONTEND=/app/dist

VOLUME [ "/config", "/data" ]

WORKDIR /app

COPY --from=server /app/target/x86_64-unknown-linux-musl/release/konarr-server /app/konarr-server
COPY --from=server /app/server/Rocket.toml /app/Rocket.toml
COPY --from=web /app/frontend/dist /app/dist
COPY --from=tools /usr/local/bin/* /usr/local/bin

ENTRYPOINT ["/app/konarr-server"]

CMD ["--config", "/config/konarr.yml"]

