# Build the Linux package first with `bash docker/build-linux.sh`.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system tspserver \
    && useradd --system --gid tspserver --home-dir /app tspserver

WORKDIR /app

COPY dist/tspserver/tspserver /usr/local/bin/tspserver
COPY routes/ /app/routes/
COPY public/ /app/public/

RUN chmod +x /usr/local/bin/tspserver \
    && chown -R tspserver:tspserver /app

ENV TSP_PORT=9000 \
    TSP_ROUTES_DIR=/app/routes \
    TSP_PUBLIC_DIR=/app/public \
    TSP_EMBEDDED_WORKER=1 \
    TSP_WORKER_COUNT=2

USER tspserver
EXPOSE 9000

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl --fail http://127.0.0.1:9000/ || exit 1

ENTRYPOINT ["/usr/local/bin/tspserver"]
