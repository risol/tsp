// Slice 17d demo: bun's native `bun:sql` (exposed as
// `tsp:server.sql`) talking to a SQLite file backing the
// `main` data source declared in `routes/_db.ts`.
//
// Endpoints that exercise the page-side connection lifecycle
// (plan §17.3):
//
//   GET  /sql_demo                  -> SELECT id, name FROM users
//                                     (table auto-created on first
//                                      call via `CREATE TABLE IF NOT
//                                      EXISTS`).
//                                     Returns 200 with the row array.
//
//   POST /sql_demo  body={name}     -> INSERT a row, return the
//                                      new id.
//
//   GET  /sql_demo?probe=pool       -> 3 sequential acquires
//                                      against the same data source
//                                      from one request handler,
//                                      proving per-worker pool
//                                      reuse. Returns 200 with the
//                                      count and pool marker.
//
//   GET  /sql_demo?source=analytics -> hit the `analytics` data
//                                      source (different sqlite
//                                      file) per the sibling
//                                      multi-datasource config
//                                      in `routes/_db.ts`. Returns
//                                      200 with the events table
//                                      contents.
//
// The router matches exact paths (no sub-path dispatch yet),
// so the multi-endpoint demo uses a `?probe=` / `?source=`
// query parameter instead of `/pool` / `multi` path suffixes.
// Once the router grows prefix dispatch (slice 7+), the page
// can switch to `ctx.url.pathname.endsWith("/pool")` style.
import { sql } from "tsp:server";
import { main as mainDb, analytics as analyticsDb } from "./_db";

async function withPool(db, handler) {
  // `sql(url)` is the factory call (returns a connection-
  // function from bun's per-worker pool). `sql\`url\`` would
  // be a QUERY, not a connect -- the page must pass the url
  // as a function argument, not as a tagged template. The
  // `try/finally` guarantees the conn returns to the pool
  // even if the handler throws -- without it, a thrown
  // handler would leak a pool slot until the worker recycled.
  const conn = await sql(db.url);
  try {
    return await handler(conn);
  } finally {
    conn.close();
  }
}

export async function GET(_ctx) {
  const source = _ctx.query.get("source") || "main";
  const probe = _ctx.query.get("probe");
  const db = source === "analytics" ? analyticsDb : mainDb;
  if (probe === "pool") {
    return withPool(db, async (conn) => {
      const r1 = await conn`SELECT COUNT(*) AS n FROM users`;
      const r2 = await conn`SELECT COUNT(*) AS n FROM users`;
      const r3 = await conn`SELECT COUNT(*) AS n FROM users`;
      return new Response(
        JSON.stringify({
          pool: source,
          rows: [r1[0], r2[0], r3[0]],
          source,
        }),
        {
          status: 200,
          headers: { "content-type": "application/json", "x-demo": "slice17d" },
        }
      );
    });
  }
  if (source === "analytics") {
    return withPool(db, async (conn) => {
      await conn`CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL, n INTEGER NOT NULL DEFAULT 0)`;
      const rows = await conn`SELECT id, kind, n FROM events ORDER BY id ASC`;
      return new Response(
        JSON.stringify({ events: rows, source }),
        {
          status: 200,
          headers: { "content-type": "application/json", "x-demo": "slice17d" },
        }
      );
    });
  }
  return withPool(db, async (conn) => {
    await conn`CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)`;
    const rows = await conn`SELECT id, name FROM users ORDER BY id ASC`;
    return new Response(JSON.stringify({ users: rows, source }), {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice17d" },
    });
  });
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  let parsed;
  try {
    parsed = JSON.parse(body);
  } catch {
    return new Response(JSON.stringify({ ok: false, error: "body must be JSON" }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
  }
  if (typeof parsed.name !== "string" || !parsed.name) {
    return new Response(
      JSON.stringify({ ok: false, error: "name must be a non-empty string" }),
      { status: 400, headers: { "content-type": "application/json" } }
    );
  }
  return withPool(mainDb, async (conn) => {
    await conn`CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)`;
    const [{ id }] = await conn`INSERT INTO users (name) VALUES (${parsed.name}) RETURNING id`;
    return new Response(JSON.stringify({ ok: true, id, name: parsed.name }), {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice17d" },
    });
  });
}
