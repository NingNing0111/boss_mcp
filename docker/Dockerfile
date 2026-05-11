FROM rust:1.91-bookworm AS builder

ARG HTTP_PROXY
ARG HTTPS_PROXY
ARG ALL_PROXY
ENV HTTP_PROXY=${HTTP_PROXY}
ENV HTTPS_PROXY=${HTTPS_PROXY}
ENV ALL_PROXY=${ALL_PROXY}

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY docker/config.yaml ./config.yaml

RUN cargo build --release --bin boss_mcp && \
    find /app/target/release -maxdepth 1 -type f -name "*.d" -delete

FROM debian:bookworm-slim

ARG HTTP_PROXY
ARG HTTPS_PROXY
ARG ALL_PROXY
ENV HTTP_PROXY=${HTTP_PROXY}
ENV HTTPS_PROXY=${HTTPS_PROXY}
ENV ALL_PROXY=${ALL_PROXY}

RUN sed -i 's|http://deb.debian.org|http://mirrors.tuna.tsinghua.edu.cn|g; s|https://deb.debian.org|http://mirrors.tuna.tsinghua.edu.cn|g; s|http://security.debian.org/debian-security|http://mirrors.tuna.tsinghua.edu.cn/debian-security|g; s|https://security.debian.org/debian-security|http://mirrors.tuna.tsinghua.edu.cn/debian-security|g' /etc/apt/sources.list.d/debian.sources \
    && apt-get -o Acquire::Retries=8 -o Acquire::http::Timeout=30 update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        bash \
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

COPY --from=builder /app/target/release/boss_mcp /app/boss_mcp
COPY --from=builder /app/config.yaml /app/config.yaml

EXPOSE 8080

CMD ["bash", "-lc", "Xvfb :99 -screen 0 1280x1024x24 & exec /app/boss_mcp"]

