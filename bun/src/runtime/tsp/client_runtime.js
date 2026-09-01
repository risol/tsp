(() => {
  "use strict";

  const requestAttributes = [
    ["GET", "data-tsp-get", "hx-get"],
    ["POST", "data-tsp-post", "hx-post"],
    ["PUT", "data-tsp-put", "hx-put"],
    ["PATCH", "data-tsp-patch", "hx-patch"],
    ["DELETE", "data-tsp-delete", "hx-delete"],
  ];

  function requestFor(element) {
    for (const [method, tspAttribute, hxAttribute] of requestAttributes) {
      const url = element.getAttribute(tspAttribute) ?? element.getAttribute(hxAttribute);
      if (url) return { method, url };
    }
    return null;
  }

  function targetFor(element) {
    const selector =
      element.getAttribute("data-tsp-target") ?? element.getAttribute("hx-target");
    if (!selector) return null;
    if (selector === "this") return element;
    try {
      return document.querySelector(selector);
    } catch {
      return null;
    }
  }

  function swapFor(element) {
    return (
      element.getAttribute("data-tsp-swap") ??
      element.getAttribute("hx-swap") ??
      "innerHTML"
    );
  }

  function formFor(element) {
    if (element instanceof HTMLFormElement) return element;
    return element.closest("form");
  }

  function urlWithFormData(url, form) {
    const target = new URL(url, window.location.href);
    if (!form) return target.toString();
    for (const [name, value] of new FormData(form).entries()) {
      if (typeof value === "string") target.searchParams.append(name, value);
    }
    return target.toString();
  }

  function setBusy(element, busy) {
    if (busy) {
      element.setAttribute("aria-busy", "true");
      element.setAttribute("data-tsp-busy", "");
      if ("disabled" in element) element.disabled = true;
    } else {
      element.removeAttribute("aria-busy");
      element.removeAttribute("data-tsp-busy");
      if ("disabled" in element) element.disabled = false;
    }
  }

  function swap(target, html, mode) {
    if (mode === "none") return;
    if (mode === "outerHTML") {
      target.insertAdjacentHTML("afterend", html);
      target.remove();
      return;
    }
    if (mode === "beforebegin" || mode === "afterend") {
      target.insertAdjacentHTML(mode, html);
      return;
    }
    if (mode === "afterbegin" || mode === "prepend") {
      target.insertAdjacentHTML("afterbegin", html);
      return;
    }
    if (mode === "beforeend" || mode === "append") {
      target.insertAdjacentHTML("beforeend", html);
      return;
    }
    target.innerHTML = html;
  }

  async function perform(element, request, target) {
    const form = formFor(element);
    const options = {
      method: request.method,
      credentials: "same-origin",
      headers: { "X-TSP-Request": "fragment" },
    };

    if (request.method === "GET") {
      request.url = urlWithFormData(request.url, form);
    } else if (form) {
      // Let fetch set the multipart boundary when a form contains files.
      options.body = new FormData(form);
    }

    const before = new CustomEvent("tsp:before-request", {
      bubbles: true,
      cancelable: true,
      detail: { element, request, target },
    });
    if (!element.dispatchEvent(before)) return;

    setBusy(element, true);
    try {
      const response = await fetch(request.url, options);
      const html = await response.text();
      if (!response.ok) {
        throw new Error(`TSP request failed with ${response.status}`);
      }
      swap(target, html, swapFor(element));
      element.dispatchEvent(
        new CustomEvent("tsp:after-request", {
          bubbles: true,
          detail: { element, request, target, response },
        }),
      );
    } catch (error) {
      element.dispatchEvent(
        new CustomEvent("tsp:request-error", {
          bubbles: true,
          detail: { element, request, target, error },
        }),
      );
      console.error(error);
    } finally {
      setBusy(element, false);
    }
  }

  function activationElement(event) {
    if (!(event.target instanceof Element)) return null;
    return event.target.closest(
      "[data-tsp-get], [data-tsp-post], [data-tsp-put], [data-tsp-patch], [data-tsp-delete], " +
        "[hx-get], [hx-post], [hx-put], [hx-patch], [hx-delete]",
    );
  }

  document.addEventListener("click", (event) => {
    if (event.defaultPrevented || event.button !== 0) return;
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const element = activationElement(event);
    if (!element) return;
    const request = requestFor(element);
    const target = targetFor(element);
    if (!request || !target) return;
    event.preventDefault();
    void perform(element, request, target);
  });

  document.addEventListener("submit", (event) => {
    const form = event.target;
    if (!(form instanceof HTMLFormElement)) return;
    const request = requestFor(form);
    const target = targetFor(form);
    if (!request || !target) return;
    event.preventDefault();
    void perform(form, request, target);
  });

  window.TSP = Object.freeze({ request: perform });
})();
