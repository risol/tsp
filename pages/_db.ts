// Shared datasource configuration for TSP routes.
//
// Plan §17.1: the host runtime does NOT own database credentials.
// Data source info lives in the page source tree (this file),
// so each page that needs DB access imports from here. Passwords
// (or full DSNs) come from process env so the source tree never
// holds a secret in plain text:
//
//   TSP_DB_MAIN_URL      full DSN (highest priority)
//   MAIN_DB_USER / MAIN_DB_PW / MAIN_DB_HOST / ...
//   (env-driven fallback used when TSP_DB_*_URL is absent)
//
// Per-request lifecycle is the page's responsibility (plan
// §17.3): the page calls `sql\`url\`` to take a connection from
// bun's per-worker pool, uses it, and calls `conn.close()` to
// return it. The connection's logical lifetime is per request;
// the underlying TCP socket is reused by the pool (PHP-FPM
// `pconnect` semantics), which is the only way the pool pays
// for itself when the BUG-0001 fix re-evaluates the page
// module on every request.
//
// `__tspServer.sql` is bun's `Bun.SQL` factory function (slice
// 17d, native Rust driver; zero prelude bytes, no mysql2
// embed). The factory produces a fresh connection object on
// every call; all of them draw from the same per-worker pool
// inside the bun runtime, so multiple pages hitting the same
// URL share a single pool transparently.
export const main = {
  url:
    process.env.TSP_DB_MAIN_URL ||
    "sqlite://" + (process.env.TSP_DB_MAIN_FILE || "/tmp/tspserver-main.db"),
  pool: 10,
};

export const orders = {
  url:
    process.env.TSP_DB_ORDERS_URL ||
    "sqlite://" + (process.env.TSP_DB_ORDERS_FILE || "/tmp/tspserver-orders.db"),
  pool: 5,
};

export const analytics = {
  url:
    process.env.TSP_DB_ANALYTICS_URL ||
    "sqlite://" +
      (process.env.TSP_DB_ANALYTICS_FILE || "/tmp/tspserver-analytics.db"),
  pool: 3,
};
