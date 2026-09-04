import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import vm from "node:vm";

const source = await readFile(new URL("../src/bootstrap.js", import.meta.url), "utf8");

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
