(function () {
  "use strict";
  globalThis.TSP_RUNTIME_ABI_VERSION = 1;
  class TspSearchParams {
    constructor(value) {
      this.values = new Map();
      for (const part of String(value || "").replace(/^\?/, "").split("&")) {
        if (!part) continue;
        const [key, ...rest] = part.split("=");
        this.values.set(decodeURIComponent(key), decodeURIComponent(rest.join("=") || ""));
      }
    }
    get(name) { return this.values.has(name) ? this.values.get(name) : null; }
    has(name) { return this.values.has(name); }
  }
  class TspUrl {
    constructor(target) {
      const value = String(target || "/");
      const queryIndex = value.indexOf("?");
      this.pathname = queryIndex < 0 ? value : value.slice(0, queryIndex);
      this.search = queryIndex < 0 ? "" : value.slice(queryIndex);
      this.searchParams = new TspSearchParams(this.search);
      this.href = value;
    }
  }
  class TspResponse {
    constructor(body = "", init = {}) {
      this.status = Number(init.status || 200);
      this.headers = Object.entries(init.headers || {});
      this.body = body == null ? "" : String(body);
    }
    toJSON() { return { status: this.status, headers: this.headers, body: this.body }; }
  }
  function escapeHtml(value) {
    return String(value).replace(/[&<>\"']/g, (character) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;"
    })[character]);
  }
  function renderChild(value) {
    if (value == null || value === false) return "";
    if (Array.isArray(value)) return value.map(renderChild).join("");
    return escapeHtml(value);
  }
  globalThis.__tsp_jsx = function (type, props, ...children) {
    if (typeof type === "function") return type({ ...(props || {}), children });
    const attributes = Object.entries(props || {})
      .filter(([key, value]) => key !== "children" && value != null && value !== false)
      .map(([key, value]) => ` ${key}="${escapeHtml(value)}"`).join("");
    return `<${type}${attributes}>${children.map(renderChild).join("")}</${type}>`;
  };
  globalThis.__tsp_fragment = (_props, ...children) => children.map(renderChild).join("");
  globalThis.Response = TspResponse;
  globalThis.process = { env: Object.create(null) };
  globalThis.__tsp_make_context = function (raw) {
    const request = raw.request || {};
    const body = String(request.body || "");
    raw.url = new TspUrl(raw.target);
    raw.query = raw.url.searchParams;
    raw.path = raw.url.pathname;
    const cookieValues = new Map();
    const setCookies = [];
    const cookieHeader = (raw.request.headers || {}).cookie || "";
    for (const pair of cookieHeader.split(";")) {
      const separator = pair.indexOf("=");
      if (separator > 0) cookieValues.set(pair.slice(0, separator).trim(), pair.slice(separator + 1).trim());
    }
    raw.cookies = {
      get: (name) => cookieValues.get(name),
      has: (name) => cookieValues.has(name),
      set: (name, value, options = {}) => {
        cookieValues.set(name, String(value));
        let line = `${name}=${encodeURIComponent(String(value))}`;
        if (options.path) line += `; Path=${options.path}`;
        if (options.maxAge != null) line += `; Max-Age=${Number(options.maxAge)}`;
        if (options.httpOnly) line += "; HttpOnly";
        if (options.secure) line += "; Secure";
        setCookies.push(line);
      },
      delete: (name) => {
        cookieValues.delete(name);
        setCookies.push(`${name}=; Max-Age=0; Path=/`);
      },
    };
    raw.__tsp_set_cookies = setCookies;
    raw.services = raw.services || Object.create(null);
    raw.signal = { aborted: false, addEventListener: () => {} };
    raw.session = raw.session || {
      id: "native-session",
      get: () => undefined,
      set: () => {},
      delete: () => {},
      regenerate: async () => {},
      destroy: async () => {},
    };
    raw.fragment = () => "";
    raw.request = {
      method: request.method,
      headers: request.headers || {},
      text: async () => body,
      json: async () => JSON.parse(body),
    };
    return raw;
  };
  globalThis.__tsp_builtin_modules = {
    "tsp:server": {
      Response: TspResponse,
      json: (value, status = 200, headers = {}) => new TspResponse(JSON.stringify(value), {
        status, headers: { "content-type": "application/json", ...headers }
      }),
      text: (value, status = 200, headers = {}) => new TspResponse(value, { status, headers }),
      html: (value, status = 200, headers = {}) => new TspResponse(value, {
        status, headers: { "content-type": "text/html; charset=utf-8", ...headers }
      }),
      redirect: (location, status = 302) => new TspResponse("", {
        status, headers: { location }
      }),
      notFound: (message = "Not Found") => new TspResponse(message, { status: 404 }),
      fragment: (handler) => handler,
      nanoid: () => "native-nanoid",
    },
    "tsp:html": { escapeHtml },
  };
})();
