/**
 * htmx helper unit tests.
 * Pure-function path: hxUrl
 * Tree-walker path: resolveHtmxFragments
 */

import {
  assertEquals,
  assertExists,
} from "./asserts.ts";
import { test } from "bun:test";
import {
  HtmxFragment,
  hxUrl,
  resolveHtmxFragments,
} from "../../src/htmx-helpers.ts";
import { createElement } from "react";

test("hxUrl: appends .tsp when missing", () => {
  assertEquals(hxUrl("/users", "table"), "/users.tsp/__fragment/table");
});

test("hxUrl: keeps .tsp when already present", () => {
  assertEquals(hxUrl("/users.tsp", "table"), "/users.tsp/__fragment/table");
});

test("hxUrl: prepends / when missing", () => {
  assertEquals(hxUrl("users", "row-1"), "/users.tsp/__fragment/row-1");
});

test("hxUrl: accepts underscores and dashes in name", () => {
  assertEquals(hxUrl("/x", "user_table"), "/x.tsp/__fragment/user_table");
  assertEquals(hxUrl("/x", "row-1"), "/x.tsp/__fragment/row-1");
});

test("hxUrl: throws on empty name", () => {
  let threw = false;
  try {
    hxUrl("/x", "");
  } catch {
    threw = true;
  }
  assertEquals(threw, true);
});

// ============================================
// resolveHtmxFragments
// ============================================

test("resolveHtmxFragments: passes primitives through", async () => {
  assertEquals(await resolveHtmxFragments(null, {}), null);
  assertEquals(await resolveHtmxFragments(undefined, {}), undefined);
  assertEquals(await resolveHtmxFragments("text", {}), "text");
  assertEquals(await resolveHtmxFragments(42, {}), 42);
  assertEquals(await resolveHtmxFragments(false, {}), false);
});

test("resolveHtmxFragments: walks arrays", async () => {
  const out = await resolveHtmxFragments(
    ["a", null, 1, ["nested", "x"]],
    {},
  ) as unknown[];
  assertEquals(out, ["a", null, 1, ["nested", "x"]]);
});

test("resolveHtmxFragments: auto-fetches fragment when no children", async () => {
  const calls: string[] = [];
  const ctx = {
    _fragments: {
      table: async (_c: unknown) => {
        calls.push("table");
        return createElement("table", { id: "t" });
      },
    },
  };
  const jsx = createElement(HtmxFragment, {
    page: "/users",
    name: "table",
    trigger: "every 5s",
  });

  const out = await resolveHtmxFragments(jsx, ctx) as {
    type: unknown;
    props: { children: unknown; page: string; name: string; trigger: string };
  };
  assertEquals(calls, ["table"]);
  // The HtmxFragment element is preserved (it's a React component),
  // and its children are now the fragment's return value.
  assertEquals(out.type, HtmxFragment);
  assertEquals(out.props.page, "/users");
  assertEquals(out.props.name, "table");
  assertEquals(out.props.trigger, "every 5s");
  const child = out.props.children as { type: string; props: { id: string } };
  assertEquals(child.type, "table");
  assertEquals(child.props.id, "t");
});

test("resolveHtmxFragments: respects explicit children", async () => {
  const calls: string[] = [];
  const ctx = {
    _fragments: {
      table: async () => {
        calls.push("table");
        return "should-not-render";
      },
    },
  };
  const jsx = createElement(
    HtmxFragment,
    { page: "/u", name: "table" },
    createElement("span", null, "override"),
  );

  const out = await resolveHtmxFragments(jsx, ctx) as {
    props: { children: { type: string; props: { children: string } } };
  };
  assertEquals(calls.length, 0, "explicit children must skip fragments lookup");
  assertEquals(out.props.children.type, "span");
  assertEquals(out.props.children.props.children, "override");
});

test("resolveHtmxFragments: warns and returns null when fragment missing", async () => {
  // Suppress the warn so test output stays clean; we only care about
  // the value returned in this test.
  const originalWarn = console.warn;
  console.warn = () => {};
  try {
    const ctx = { _fragments: {} };
    const jsx = createElement(HtmxFragment, { page: "/u", name: "missing" });
    const out = await resolveHtmxFragments(jsx, ctx) as {
      props: { children: unknown };
    };
    assertEquals(out.props.children, null);
  } finally {
    console.warn = originalWarn;
  }
});

test("resolveHtmxFragments: recurses into nested children", async () => {
  const ctx = {
    _fragments: {
      inner: async () => createElement("em", null, "from-fragment"),
    },
  };
  const jsx = createElement(
    "section",
    null,
    createElement("h1", null, "title"),
    createElement(HtmxFragment, { page: "/u", name: "inner" }),
    createElement("p", null, "footer"),
  );

  const out = await resolveHtmxFragments(jsx, ctx) as {
    type: string;
    props: { children: unknown[] };
  };
  assertEquals(out.type, "section");
  const kids = out.props.children as Array<
    { type: unknown; props: { children?: unknown } }
  >;
  assertEquals(kids.length, 3);
  // The walker replaces the HtmxFragment's children with the fragment
  // function's return value. The wrapper component itself stays in the
  // tree so React will render it as the hx-* div on the next pass.
  assertEquals(kids[1].type, HtmxFragment);
  const innerEm = kids[1].props.children as {
    type: string;
    props: { children: string };
  };
  assertEquals(innerEm.type, "em");
  assertEquals(innerEm.props.children, "from-fragment");
});
