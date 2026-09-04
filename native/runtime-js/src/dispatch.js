(function () {
  "use strict";

  function normalize(value, context) {
    const response = value instanceof Response ? value : new Response(value);
    for (const cookie of context.__tsp_set_cookies || []) {
      response.headers.push(["set-cookie", cookie]);
    }
    return response;
  }

  // This function is compiled once per worker. Request data arrives as a
  // value through the engine adapter, so the hot path never creates source
  // code from an HTTP request.
  globalThis.__tsp_dispatch_json = function (input) {
    globalThis.__tsp_pending = true;
    globalThis.__tsp_result = undefined;
    globalThis.__tsp_error = undefined;
    globalThis.__tsp_request_id = input.request_id || "";
    try {
      const route = globalThis.__tsp_routes && globalThis.__tsp_routes[input.route];
      const handler = route && (route[input.method] || route.ANY);
      if (!handler) {
        globalThis.__tsp_result = new Response("Method Not Allowed", { status: 405 });
        globalThis.__tsp_pending = false;
        return "scheduled";
      }
      const context = globalThis.__tsp_make_context({
        method: input.method,
        target: input.target,
        params: input.params || {},
        request: input.request || {},
      });
      const value = handler(context);
      if (value && typeof value.then === "function") {
        value.then(
          (resolved) => {
            globalThis.__tsp_result = normalize(resolved, context);
            globalThis.__tsp_pending = false;
          },
          (error) => {
            globalThis.__tsp_error = String(error && error.stack || error);
            globalThis.__tsp_pending = false;
          },
        );
      } else {
        globalThis.__tsp_result = normalize(value, context);
        globalThis.__tsp_pending = false;
      }
    } catch (error) {
      globalThis.__tsp_error = String(error && error.stack || error);
      globalThis.__tsp_pending = false;
    }
    return "scheduled";
  };

  globalThis.__tsp_read_response_json = function () {
    const response = globalThis.__tsp_result;
    return JSON.stringify({
      pending: !!globalThis.__tsp_pending,
      error: globalThis.__tsp_error || null,
      result: response ? {
        version: 1,
        request_id: globalThis.__tsp_request_id || "",
        status: response.status,
        headers: response.headers,
        body: { kind: "Text", data: response.body },
        effects: { cookies: [], session: [] },
      } : null,
    });
  };
})();
