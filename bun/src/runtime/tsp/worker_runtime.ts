// Persistent TSP worker bootstrap.
//
// The native host sends one generated .tsx wrapper per request. Bun owns
// importing and evaluating that wrapper; the host only receives the existing
// __TSP_OUT_V1__ response envelope. The protocol is intentionally tiny for
// the first vertical slice and is length-safe because every field is a single
// line and paths are percent-encoded by the Rust side.

const PREFIX = "__TSP_WORKER_V1__";
const originalExit = process.exit.bind(process);
const originalConsoleLog = console.log.bind(console);
const originalConsoleError = console.error.bind(console);
const originalStdinOn = process.stdin.on.bind(process.stdin);

function send(kind: string, id?: string, payload?: string): void {
  let line = PREFIX + kind;
  if (id !== undefined) line += "\t" + id + "\t" + (payload ?? "");
  process.stdout.write(line + "\n");
}

send("ready");

let activeId: string | undefined;
let activeAbort: (() => void) | undefined;
let input = "";

async function execute(id: string, encodedPath: string): Promise<void> {
  if (activeId !== undefined) {
    send("error", id, "worker is busy");
    return;
  }

  activeId = id;
  activeAbort = undefined;

  const response = new Promise<string>((resolve, reject) => {
    let settled = false;
    const finish = (fn: (value: string) => void, value: string) => {
      if (settled) return;
      settled = true;
      fn(value);
    };

    // The generated one-shot wrapper calls console.log with the response
    // marker and then calls process.exit(0). Intercept both so the process
    // stays alive and the envelope becomes the result of this request.
    console.log = (...args: unknown[]) => {
      const first = String(args[0] ?? "");
      if (first.startsWith("__TSP_OUT_V1__\n")) {
        finish(resolve, first.slice("__TSP_OUT_V1__\n".length));
      } else {
        originalConsoleError(...args);
      }
    };
    console.error = (...args: unknown[]) => originalConsoleError(...args);
    process.exit = ((code?: number) => {
      if (code && code !== 0) finish(reject as unknown as (value: string) => void, String(code));
    }) as typeof process.exit;

    // The one-shot wrapper installs a stdin data listener to implement
    // AbortSignal. Capture that callback instead; the worker's own protocol
    // reader remains the only real stdin listener.
    process.stdin.on = ((event: string, listener: (...args: unknown[]) => void) => {
      if (event === "data") {
        activeAbort = () => listener(Buffer.from("A\n"));
        return process.stdin;
      }
      return originalStdinOn(event, listener);
    }) as typeof process.stdin.on;

    const decodedPath = decodeURIComponent(encodedPath).replaceAll("\\", "/");
    const fileUrl = new URL(
      decodedPath.startsWith("/") ? "file://" + decodedPath : "file:///" + decodedPath,
    );
    fileUrl.search = "?tsp_worker_request=" + encodeURIComponent(id);

    import(fileUrl.href).catch((error: unknown) => {
      finish(reject as unknown as (value: string) => void, String(error instanceof Error ? error.stack || error : error));
    });
  });

  try {
    const envelope = await response;
    send("response", id, envelope);
  } catch (error) {
    send("error", id, String(error));
  } finally {
    console.log = originalConsoleLog;
    console.error = originalConsoleError;
    process.exit = originalExit;
    process.stdin.on = originalStdinOn as typeof process.stdin.on;
    activeAbort = undefined;
    activeId = undefined;
  }
}

function handleLine(line: string): void {
  const parts = line.trimEnd().split("\t");
  const kind = parts[0];
  const id = parts[1];
  if (kind === "execute" && id && parts[2]) {
    void execute(id, parts[2]);
  } else if (kind === "cancel" && id && id === activeId) {
    try {
      activeAbort?.();
    } catch {
      // Cancellation is best effort; the native host owns the hard timeout.
    }
  } else if (kind !== "execute" && kind !== "cancel") {
    send("error", id || "0", "unknown worker command");
  }
}

process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk: string) => {
  input += chunk;
  let newline = input.indexOf("\n");
  while (newline >= 0) {
    const line = input.slice(0, newline);
    input = input.slice(newline + 1);
    if (line) handleLine(line);
    newline = input.indexOf("\n");
  }
});

process.stdin.on("end", () => {
  if (activeId === undefined) originalExit(0);
});
