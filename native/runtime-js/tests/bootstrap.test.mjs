import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import vm from "node:vm";

const source = await readFile(new URL("../src/bootstrap.js", import.meta.url), "utf8");
const dispatchSource = await readFile(new URL("../src/dispatch.js", import.meta.url), "utf8");

test("bootstrap publishes the versioned runtime surface", () => {
  const context = { console };
  vm.runInNewContext(source, context, { filename: "tsp-runtime.js" });
  assert.equal(context.TSP_RUNTIME_ABI_VERSION, 1);
  assert.equal(typeof context.__tsp_make_context, "function");
  assert.equal(typeof context.__tsp_builtin_modules["tsp:server"].json, "function");
});

test("bootstrap builds request context without host interpolation", async () => {
  const context = { console };
  vm.runInNewContext(source, context, { filename: "tsp-runtime.js" });
  const result = context.__tsp_make_context({
    target: "/users/42?q=hello",
    request: { method: "GET", headers: { cookie: "sid=abc" }, body: "" },
  });
  assert.equal(result.path, "/users/42");
  assert.equal(result.query.get("q"), "hello");
  assert.equal(result.cookies.get("sid"), "abc");
  assert.equal(await result.request.text(), "");
});

test("dispatch is a cached function that accepts structured request data", () => {
  const context = { console };
  vm.runInNewContext(source, context, { filename: "tsp-runtime.js" });
  context.__tsp_routes = {
    "/hello": {
      GET: (request) => new context.Response(`hello ${request.params.name}`),
    },
  };
  vm.runInNewContext(dispatchSource, context, { filename: "tsp-dispatch.js" });
  assert.equal(typeof context.__tsp_dispatch_json, "function");
  context.__tsp_dispatch_json({
    route: "/hello",
    request_id: "test-1",
    method: "GET",
    target: "/hello/world",
    params: { name: "world" },
    request: { method: "GET", headers: {}, body: "" },
  });
  const result = JSON.parse(context.__tsp_read_response_json());
  assert.equal(result.pending, false);
  assert.equal(result.result.body.data, "hello world");
  assert.equal(result.result.request_id, "test-1");
});

test("dispatch exposes an explicit cancellation boundary", () => {
  const context = { console };
  vm.runInNewContext(source, context, { filename: "tsp-runtime.js" });
  vm.runInNewContext(dispatchSource, context, { filename: "tsp-dispatch.js" });
  context.__tsp_pending = true;
  context.__tsp_cancel();
  const result = JSON.parse(context.__tsp_read_response_json());
  assert.equal(result.pending, false);
  assert.equal(result.error, "request cancelled");
});
