FROM docker.io/library/rust:1.80-alpine as builder

WORKDIR /app

COPY . .

ENV OPENSSL_INCLUDE_DIR=/usr/include/ssl
RUN apk add --no-cache openssl-dev musl-dev && \
    rustup target add x86_64-unknown-linux-musl && \
    cargo build -p konarr-server --release --target x86_64-unknown-linux-musl

FROM docker.io/library/alpine:3.20

EXPOSE 9000
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=9000

VOLUME [ "/config", "/data" ]

WORKDIR /app

COPY --from=server /app/target/release/konarr-server /app/konarr-server
COPY --from=server /app/server/Rocket.toml /app/Rocket.toml
COPY --from=server /app/server/konarr.yml /config/konarr.yml
COPY --from=server /app/dist /app/dist

ENTRYPOINT ["/app/konarr-server"]
CMD ["--config", "/config/konarr.yml"]


