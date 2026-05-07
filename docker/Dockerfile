FROM rust:1.86-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY config.yaml ./config.yaml

RUN cargo build --release --bin login --bin login_check && \
    find /app/target/release -maxdepth 1 -type f -name "*.d" -delete

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && sed -i 's|http://deb.debian.org|https://deb.debian.org|g' /etc/apt/sources.list.d/debian.sources \
    && apt-get -o Acquire::Retries=5 update \
    && apt-get -o Acquire::Retries=5 install -y --no-install-recommends \
        chromium \
        xvfb \
        libglib2.0-0 \
        libnss3 \
        libatk-bridge2.0-0 \
        libatk1.0-0 \
        libcups2 \
        libdbus-1-3 \
        libxkbcommon0 \
        libxcomposite1 \
        libxdamage1 \
        libxfixes3 \
        libxrandr2 \
        libgbm1 \
        libasound2 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

ENV DISPLAY=:99

WORKDIR /app

RUN mkdir -p /data/qr /data/session

COPY --from=builder /app/target/release/login /app/login
COPY --from=builder /app/target/release/login_check /app/login_check
COPY config.yaml /app/config.yaml

CMD ["bash", "-lc", "Xvfb :99 -screen 0 1280x1024x24 & trap 'kill 0' EXIT; tail -f /dev/null"]
