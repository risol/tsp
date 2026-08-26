# Build the v2 package first with `./tsp.sh build`.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system tspserver \
    && useradd --system --gid tspserver --home-dir /app tspserver

WORKDIR /app

COPY dist/tsp-v2/tspserver_v2 /usr/local/bin/tspserver_v2
COPY routes/ /app/routes/
COPY public/ /app/public/

RUN chmod +x /usr/local/bin/tspserver_v2 \
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

ENTRYPOINT ["/usr/local/bin/tspserver_v2"]
