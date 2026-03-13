#!/usr/bin/env -S deno-tsp run --allow-net --allow-read

/**
 * TSP: TypeScript Server Page
 * Template server implemented using Deno + TSX
 */

import { resolvePath, securityCheck } from "./router.ts";
import { buildContext } from "./context.ts";
import { renderToString } from "react-dom/server";
import { registerDep } from "./injection-typed.ts";
import { serveStaticFileWithCache } from "./static.ts";
import { dirname, join, relative, resolve } from "std/path";
import { createMySQL } from "./mysql/factory.ts";
import { createRedis } from "./redis/factory.ts";
import { createLdapClient } from "./ldap/client.ts";
import { createExcelJS as createExcelJSImport } from "./exceljs/factory.ts";
import { parse as parseJsonc } from "jsr:/@std/jsonc";
import {
  createSessionManager,
  getDefaultOptions,
  RedisSessionStore,
  SessionStore,
  type SessionStoreInterface,
} from "./session.ts";
import { createCookieManager } from "./cookies.ts";
import { createResponseHelper } from "./response.ts";
import { parseMultipartFormData, type UploadedFile } from "./files.ts";
import {
  createDefaultLogger,
  createProductionLogger,
  type Logger,
} from "./logger.ts";
import type { FileManagerConfig } from "./filemanager/types.ts";
import { validateFileManagerConfig } from "./filemanager/config.ts";
import { TSP_VERSION } from "./version.ts";
import { LogRotator } from "./logger-rotation.ts";

// Session config interface
export interface SessionConfig {
  /** Cookie name (default: 'tsp_session') */
  cookieName?: string;
  /** Session validity period in seconds (default: 86400 = 1 day) */
  maxAge?: number;
  /** Whether to add Secure attribute (default: true, recommended in HTTPS) */
  secure?: boolean;
  /** Whether to add HttpOnly attribute (default: true) */
  httpOnly?: boolean;
  /** SameSite attribute (default: 'Strict') */
  sameSite?: "Strict" | "Lax" | "None";
  /** Cookie path (default: '/') */
  path?: string;
  /** Whether to enable rolling session (refresh expiry on each visit, default: true) */
  rolling?: boolean;
}

// Redis config interface
export interface RedisConfig {
  /** Redis host (default: '127.0.0.1') */
  host?: string;
  /** Redis port (default: 6379) */
  port?: number;
  /** Redis password (optional) */
  password?: string;
  /** Redis database number (default: 0) */
  db?: number;
}

// Logger config interface
export interface LoggerConfig {
  /** Minimum log level: DEBUG, INFO, WARN, ERROR */
  level?: "DEBUG" | "INFO" | "WARN" | "ERROR";
  /** Log file path (optional) */
  file?: string;
  /** Whether to enable colorized output */
  colorize?: boolean;
  /** Log format: text or json */
  format?: "text" | "json";
  /** Log rotation configuration */
  rotation?: {
    /** Maximum size of single log file (bytes), default 10MB */
    maxSize?: number;
    /** Number of archived files to keep, default 5 */
    maxFiles?: number;
    /** Whether to compress archived files (gzip), default false */
    compress?: boolean;
    /** Daily rotation: create new file each day */
    daily?: boolean;
  };
}

// Access log config interface
export interface AccessLogConfig {
  /** Log file path (optional, if not set logs to stdout) */
  file?: string;
  /** Log rotation configuration */
  rotation?: {
    /** Maximum size of single log file (bytes), default 10MB */
    maxSize?: number;
    /** Number of archived files to keep, default 5 */
    maxFiles?: number;
    /** Whether to compress archived files (gzip), default false */
    compress?: boolean;
    /** Daily rotation: create new file each day */
    daily?: boolean;
  };
}

// Config interface
export interface Config {
  root: string;
  port: number;
  dev: boolean;
  /** Access log config */
  accessLog?: AccessLogConfig;
  staticExtensions?: string[];
  /** Session config */
  session?: SessionConfig;
  /** Logger config */
  logger?: LoggerConfig;
  /** File manager config */
  fileManager?: FileManagerConfig;
  /** Redis config for session sharing */
  redis?: RedisConfig;
}

// Default supported static file extensions
const DEFAULT_STATIC_EXTENSIONS = [
  ".html",
  ".htm",
  ".css",
  ".js",
  ".json",
  ".png",
  ".jpg",
  ".jpeg",
  ".gif",
  ".svg",
  ".ico",
  ".webp",
  ".woff",
  ".woff2",
  ".ttf",
  ".eot",
  ".mp3",
  ".mp4",
  ".webm",
  ".txt",
  ".md",
  ".xml",
];

// Default config
const DEFAULT_CONFIG: Config = {
  root: "./www",
  port: 9000,
  dev: false,
  accessLog: {
    file: ".logs/access.log",
  },
  staticExtensions: DEFAULT_STATIC_EXTENSIONS,
};

// Config file interface (same as Config, but all fields are optional)
interface ConfigFile {
  root?: string;
  port?: number;
  dev?: boolean;
  /** Access log config */
  accessLog?: AccessLogConfig;
  staticExtensions?: string[];
  /** Session config */
  session?: SessionConfig;
  /** Logger config */
  logger?: LoggerConfig;
  /** File manager config */
  fileManager?: FileManagerConfig;
}

/**
 * Record HTTP access log
 * @param req Request object
 * @param resp Response object
 * @param config Config object
 * @param logger Optional logger instance
 */
export async function logAccess(
  req: Request,
  resp: Response,
  config: Config,
  logger?: Logger,
): Promise<void> {
  const url = new URL(req.url);
  const now = new Date();
  const timestamp = now.toISOString();
  const method = req.method;
  const pathname = url.pathname;
  const status = resp.status;
  const userAgent = req.headers.get("user-agent") || "-";

  // Get response time (can add timing logic here if needed)
  const logLine = `${timestamp} ${method} ${pathname} ${status} "${userAgent}"`;

  const accessLogConfig = config.accessLog;

  if (accessLogConfig?.file) {
    // Initialize or update rotator if config changed
    const rotationConfig = accessLogConfig.rotation;
    const rotatorKey = `${accessLogConfig.file}-${JSON.stringify(rotationConfig)}`;

    if (!accessLogRotator) {
      accessLogRotator = new LogRotator(accessLogConfig.file, rotationConfig);
    }

    try {
      await accessLogRotator.write(logLine);
    } catch (error) {
      logger?.error("Failed to write access log", error);
    }
  } else if (!accessLogConfig?.file && accessLogConfig) {
    // Access log is configured but no file path, output to console
    console.log(logLine);
  } else {
    // No access log config, output to console (default behavior)
    console.log(logLine);
  }
}

/**
 * Load config from config file
 * @param filepath Config file path
 * @param logger Optional logger instance
 * @returns Config object, or null if file does not exist
 */
async function loadConfigFile(
  filepath: string,
  logger?: Logger,
): Promise<Config | null> {
  try {
    const content = await Deno.readTextFile(filepath);

    // If JSONC file, use Deno's built-in JSONC parser
    let config: ConfigFile;
    if (filepath.endsWith(".jsonc")) {
      config = parseJsonc(content) as ConfigFile;
    } else {
      config = JSON.parse(content);
    }

    // Validate file manager config (if exists)
    if (config.fileManager) {
      logger?.info("File manager config found", {
        enabled: config.fileManager.enabled,
      });
      const validated = validateFileManagerConfig(config.fileManager);
      logger?.info("File manager config validated", {
        enabled: validated.enabled,
        path: validated.path,
      });
      // If not enabled, remove config
      if (!validated.enabled) {
        logger?.info("File manager not enabled, removing config");
        delete config.fileManager;
      } else {
        // Replace with validated config
        logger?.info("File manager enabled and validated");
        (config as Config).fileManager = validated;
      }
    } else {
      logger?.info("File manager not configured");
    }

    // Merge default config and file config
    return {
      ...DEFAULT_CONFIG,
      ...config,
    };
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      // File does not exist, return null
      return null;
    } else if (error instanceof SyntaxError) {
      const msg = `Config file format error: ${error.message}`;
      logger?.error(msg);
      console.error(msg);
      Deno.exit(1);
    } else {
      throw error;
    }
  }
}

// Global config reload state
let currentConfig: Config | null = null;
let configFilepath: string | null = null;
let configMtime: number | null = null; // Config file modification time
let accessLogRotator: LogRotator | null = null; // Access log rotator instance

/**
 * Reload config file if it has been modified
 */
async function reloadConfigIfNeeded(logger?: Logger): Promise<Config> {
  if (!configFilepath) {
    // No config file, use current config or default config
    return currentConfig || { ...DEFAULT_CONFIG };
  }

  try {
    const stat = await Deno.stat(configFilepath);
    const newMtime = stat.mtime?.getTime() || 0;

    // If config file has not been modified, return current config
    if (configMtime !== null && newMtime === configMtime && currentConfig) {
      return currentConfig;
    }

    // Config file has been modified or first load, reload
    logger?.info("Config file modified, reloading", { path: configFilepath });
    console.log(`ℹ️  Config file modified, reloading: ${configFilepath}`);

    const loadedConfig = await loadConfigFile(configFilepath, logger);

    if (loadedConfig) {
      // Update current config and modification time
      currentConfig = loadedConfig;
      configMtime = newMtime;

      // Resolve root directory to absolute path
      // Use config file directory as base path (important for binary mode)
      const configDir = dirname(configFilepath);
      const rootPath = currentConfig.root;
      // If root is relative path, resolve relative to config file directory
      if (!rootPath.startsWith("/") && !rootPath.match(/^[a-zA-Z]:/)) {
        currentConfig.root = resolve(configDir, rootPath);
      } else {
        currentConfig.root = resolve(rootPath);
      }

      // Validate file manager config
      if (currentConfig.fileManager) {
        const validated = validateFileManagerConfig(currentConfig.fileManager);
        currentConfig.fileManager = validated;
      }

      logger?.info("Config reload successful");
      console.log("✓ Config reload successful");

      return currentConfig;
    } else {
      logger?.warn("Config file load failed, using current config");
      return currentConfig || { ...DEFAULT_CONFIG };
    }
  } catch (error) {
    logger?.error("Failed to check config file modification time", error);
    return currentConfig || { ...DEFAULT_CONFIG };
  }
}

// Parse command line arguments
async function parseArgs(logger?: Logger): Promise<Config> {
  const args = Deno.args;
  let config = { ...DEFAULT_CONFIG };
  let foundConfigPath: string | null = null;

  // First pass: find --config parameter
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "--config" || arg === "-c") {
      foundConfigPath = args[++i];
      break;
    }
  }

  // If config file is specified, load it first
  if (foundConfigPath) {
    const loadedConfig = await loadConfigFile(foundConfigPath, logger);
    if (loadedConfig) {
      config = loadedConfig;
    } else {
      console.error(`Config file not found: ${foundConfigPath}`);
      Deno.exit(1);
    }
  } else {
    // Try to automatically find default config file
    const defaultConfigFiles = [
      "config.jsonc",
      "config.json",
    ];

    let configFound = false;
    for (const filename of defaultConfigFiles) {
      try {
        await Deno.stat(filename);
        const msg = `✓ Found config file: ${filename}`;
        logger?.info(msg);
        console.log(msg);
        const loadedConfig = await loadConfigFile(filename, logger);
        if (loadedConfig) {
          config = loadedConfig;
          foundConfigPath = resolve(filename);
          configFound = true;
          break;
        }
      } catch {
        // File does not exist, continue searching
      }
    }

    // If no config file found, use default config
    if (!configFound) {
      const msg = "No config file found, using default config";
      logger?.info(msg);
      console.log(`ℹ  ${msg}`);
      // config is already initialized as DEFAULT_CONFIG, no additional action needed
    }
  }

  // Second pass: command line arguments override config file (higher priority)
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
      case "--config":
      case "-c":
        // Skip config file path (already handled in first pass)
        i++;
        break;
      case "--root":
      case "-r":
        config.root = args[++i];
        break;
      case "--port":
      case "-p":
        config.port = parseInt(args[++i], 10);
        break;
      case "--dev":
      case "-d":
        config.dev = true;
        break;
      case "--access-log":
      case "-a":
        if (!config.accessLog) {
          config.accessLog = {};
        }
        config.accessLog.file = args[++i];
        break;
      case "--help":
      case "-h":
        printHelp();
        Deno.exit(0);
      default:
        // Ignore unknown arguments, continue processing
        break;
    }
  }

  // Set global config variables and config file path
  currentConfig = config;
  configFilepath = foundConfigPath;

  // Record config file modification time
  if (configFilepath) {
    try {
      const stat = await Deno.stat(configFilepath);
      configMtime = stat.mtime?.getTime() || 0;
    } catch {
      configMtime = null;
    }
  }

  return config;
}

// Print help information
function printHelp(): void {
  console.log(`
TSP: TypeScript Server Page

Usage:
  ./tspserver [options]

Options:
  --config, -c <file>     Config file path (default: auto-find config.json)
  --root, -r <path>       Document root directory (default: ./www)
  --port, -p <port>       Listening port (default: 9000)
  --dev, -d               Development mode (show detailed errors)
  --access-log, -a <path> Access log file path (default: console output)
  --help, -h              Show help information

Config file:
  Supported config file names (by priority):
    - config.jsonc
    - config.json

  Config file format (JSON):
  {
    "root": "./www",
    "port": 9000,
    "dev": false,
    "accessLog": { "file": "./logs/access.log", "rotation": { "maxSize": 10485760, "maxFiles": 5 } },
    "staticExtensions": [".css", ".js", ".png", ".jpg"]
  }

  Priority: Command line arguments > Config file > Default values

Examples:
  # Use config file
  ./tspserver

  # Specify config file
  ./tspserver --config ./my-config.json

  # Command line arguments override config file
  ./tspserver --port 8080 --dev

  # Pure command line arguments
  ./tspserver --root ./www --port 9000 --dev

  # Record access log to file
  ./tspserver --access-log ./access.log

  # Access log output to console (default)
  ./tspserver
`);
}

// Handle request
async function handleRequest(
  req: Request,
  config: Config,
  serverLogger: Logger,
): Promise<Response> {
  let filepath = "";

  try {
    // Check and reload config if config file has been modified
    config = await reloadConfigIfNeeded(serverLogger);

    const url = new URL(req.url);
    const pathname = url.pathname;

    // File manager route interception
    if (config.fileManager?.enabled) {
      const fmPath = config.fileManager.path || "/__filemanager";
      // Ensure consistent format when comparing paths
      const normalizedFmPath = fmPath.endsWith("/")
        ? fmPath.slice(0, -1)
        : fmPath;
      const normalizedPathname = pathname.endsWith("/") && pathname !== "/"
        ? pathname.slice(0, -1)
        : pathname;

      if (
        normalizedPathname === normalizedFmPath ||
        normalizedPathname.startsWith(normalizedFmPath + "/")
      ) {
        serverLogger.debug("File manager route matched", { pathname });
        const { handleFileManagerRequest } = await import(
          "./filemanager/mod.ts"
        );
        // Ensure passing fully validated config
        const fmConfig = validateFileManagerConfig(config.fileManager);
        // Get session secure config
        const sessionSecure = config.session?.secure ?? true;
        return handleFileManagerRequest(
          req,
          fmConfig,
          config.root,
          serverLogger,
          sessionSecure,
        );
      }
    }

    // Resolve file path (including static file extensions)
    const staticExtensions = config.staticExtensions || [];
    const fileResult = resolvePath(pathname, config.root, staticExtensions);
    if (!fileResult.success) {
      return new Response(fileResult.error, {
        status: 404,
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }

    filepath = fileResult.filepath!;

    // Security check (including static file extensions)
    const securityResult = await securityCheck(
      filepath,
      config.root,
      staticExtensions,
    );
    if (!securityResult.success) {
      // Determine status code based on error type
      const error = securityResult.error || "";

      // File not found -> 404
      // Permission denied -> 403
      // Other errors -> 500
      let status = 500;
      if (
        error.includes("File not found") || error.includes("not found") ||
        error.includes("Directory index")
      ) {
        status = 404;
      } else if (
        error.includes("Access denied") ||
        error.includes("File type not allowed") ||
        error.includes("Internal file")
      ) {
        status = 403;
      }

      return new Response(error, {
        status,
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }

    // Check if it's a static file (if extension is allowed in config)
    // Static file extension list is provided in config
    const allowedExtensions = config.staticExtensions || [];
    const staticResponse = await serveStaticFileWithCache(
      filepath,
      allowedExtensions,
      req.headers,
      config.dev,
    );

    // If it's a static file, return directly
    if (staticResponse !== null) {
      return staticResponse;
    }

    // Parse request body
    let body: unknown = null;
    // Parse uploaded files
    let files: Record<string, UploadedFile | UploadedFile[]> = {};
    const contentType = req.headers.get("content-type") || "";
    if (
      req.method === "POST" || req.method === "PUT" || req.method === "PATCH"
    ) {
      if (contentType.includes("application/json")) {
        try {
          body = await req.json();
        } catch {
          body = null;
        }
      } else if (contentType.includes("application/x-www-form-urlencoded")) {
        const text = await req.text();
        body = Object.fromEntries(new URLSearchParams(text));
      } else if (contentType.includes("multipart/form-data")) {
        // Parse file upload
        try {
          const arrayBuffer = await req.arrayBuffer();
          const requestBody = new Uint8Array(arrayBuffer);
          const result = await parseMultipartFormData(
            requestBody,
            contentType,
            {
              maxSize: 10 * 1024 * 1024, // 10MB default limit
            },
          );
          body = result.fields;
          files = result.files;
        } catch (error) {
          serverLogger.error("Failed to parse multipart form data", error);
          body = null;
          files = {};
        }
      } else {
        body = await req.text();
      }
    }

    // Parse cookies
    const cookies: Record<string, string> = {};
    const cookieHeader = req.headers.get("cookie");
    if (cookieHeader) {
      for (const pair of cookieHeader.split(";")) {
        const [key, value] = pair.trim().split("=");
        if (key && value) {
          cookies[key] = decodeURIComponent(value);
        }
      }
    }

    // Parse query parameters
    const query: Record<string, string> = {};
    url.searchParams.forEach((value, key) => {
      query[key] = value;
    });

    // Build context
    const context = buildContext({
      method: req.method as
        | "GET"
        | "POST"
        | "PUT"
        | "PATCH"
        | "DELETE"
        | "HEAD"
        | "OPTIONS",
      url,
      headers: req.headers,
      query,
      body,
      cookies,
      files,
      file: filepath,
      root: config.root,
    });

    // Set React global variable (Deno built-in JSX support, needs React object)
    // Only set on first request
    if (!(globalThis as any).React) {
      const react = await import("react");
      (globalThis as any).React = react;
    }

    // Get and execute page function
    // Deno compiler has built-in .tsp transpilation support, just import directly
    let pageFn: (context: any) => Promise<any>;

    // Import page file directly (Deno compiler has built-in .tsp transpilation support)
    // Hot reload: handled by deno-tsp run --watch or compiled binary's --dynamic-import-no-cache
    const fileUrlBase = "file://" + filepath.replace(/\\/g, "/");

    // Main file URL (hot reload handled by deno-tsp run --watch or compiled binary's dynamic_import_no_cache)
    let fileUrl = fileUrlBase;

    let pageModule: any;
    try {
      pageModule = await import(fileUrl);
    } catch (importError) {
      // Re-throw with simplified message, detailed info will be in catch block
      const errMsg = importError instanceof Error ? importError.message : String(importError);
      throw new Error(`Module load failed: ${errMsg}`);
    }

    pageFn = pageModule.default;

    const result = await pageFn(context);

    // Extract cookie response headers
    const { extractSetCookieHeaders } = await import("./cookies.ts");
    const setCookieHeaders = extractSetCookieHeaders(context);

    // Check if it's a redirect object
    if (result && typeof result === "object" && "redirect" in result) {
      // Strict check: ensure it's not a VNode
      const isVNode = "type" in result || "props" in result ||
        "__k" in result || "__" in result;

      if (!isVNode) {
        const redirectResult = result as RedirectResult;
        const targetUrl = redirectResult.redirect;
        const status = redirectResult.status ?? 302;

        // Validate redirect status code
        const validStatuses = [301, 302, 303, 307, 308];
        const finalStatus = validStatuses.includes(status) ? status : 302;

        const headers: HeadersInit = { "Location": targetUrl };

        // Add Set-Cookie response header
        if (setCookieHeaders && setCookieHeaders.length > 0) {
          const cookieHeaders: string[] = [];
          for (const header of setCookieHeaders) {
            cookieHeaders.push(header);
          }
          (headers as Record<string, string | string[]>)["Set-Cookie"] =
            cookieHeaders;
        }

        return new Response(null, {
          status: finalStatus,
          headers,
        });
      }
    }

    // Check if it's a Response object (direct return)
    if (result instanceof Response) {
      // If cookies need to be set, create new Response object and add Set-Cookie header
      if (setCookieHeaders && setCookieHeaders.length > 0) {
        const newHeaders = new Headers(result.headers);
        for (const header of setCookieHeaders) {
          newHeaders.append("Set-Cookie", header);
        }
        return new Response(result.body, {
          status: result.status,
          statusText: result.statusText,
          headers: newHeaders,
        });
      }
      return result;
    }

    // Default: render JSX as HTML
    const html = "<!DOCTYPE html>\n" + renderToString(result as any);

    // Set response headers
    const headers: HeadersInit = {
      "Content-Type": "text/html; charset=utf-8",
    };

    // Development mode: disable browser caching
    if (config.dev) {
      headers["Cache-Control"] = "no-cache, no-store, must-revalidate";
      headers["Pragma"] = "no-cache";
      headers["Expires"] = "0";
    }

    // Add Set-Cookie response header
    if (setCookieHeaders && setCookieHeaders.length > 0) {
      const cookieHeaders: string[] = [];
      for (const header of setCookieHeaders) {
        cookieHeaders.push(header);
      }
      (headers as Record<string, string | string[]>)["Set-Cookie"] =
        cookieHeaders;
    }

    return new Response(html, {
      status: 200,
      headers,
    });
  } catch (error) {
    // Error handling
    const errorMessage = error instanceof Error ? error.message : String(error);
    const stackTrace = error instanceof Error ? error.stack : "";

    // Log error
    serverLogger.error("Request handling error", {
      error: errorMessage,
      filepath: filepath,
      stack: stackTrace,
      url: req.url,
      method: req.method,
    });

    if (config.dev) {
      // Development mode: show detailed error
      const errorType = error instanceof Error ? error.name : "Error";
      const html = `
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>500 - Server Error</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'SF Mono', Consolas, 'Courier New', monospace;
      background: #fff;
      color: #333;
      font-size: 13px;
      line-height: 1.5;
      padding: 24px;
    }
    .error-header {
      margin-bottom: 20px;
    }
    .error-title {
      color: #d32f2f;
      font-size: 16px;
      font-weight: 600;
      margin-bottom: 4px;
    }
    .error-type {
      color: #888;
      font-size: 12px;
    }
    .info-row {
      display: flex;
      margin-bottom: 8px;
    }
    .info-label {
      color: #666;
      width: 80px;
      flex-shrink: 0;
    }
    .info-value {
      color: #333;
      word-break: break-all;
    }
    .error-msg {
      color: #d32f2f;
      margin: 16px 0;
      white-space: pre-wrap;
      word-break: break-word;
    }
    .stack {
      color: #666;
      white-space: pre-wrap;
      word-break: break-word;
      max-height: 300px;
      overflow-y: auto;
      background: #f5f5f5;
      padding: 12px;
    }
  </style>
</head>
<body>
  <div class="error-header">
    <div class="error-title">500 Internal Server Error</div>
    <div class="error-type">${errorType}</div>
  </div>

  <div class="info-row">
    <span class="info-label">Time:</span>
    <span class="info-value">${new Date().toISOString()}</span>
  </div>
  <div class="info-row">
    <span class="info-label">URL:</span>
    <span class="info-value">${req.url}</span>
  </div>
  <div class="info-row">
    <span class="info-label">Method:</span>
    <span class="info-value">${req.method}</span>
  </div>
  <div class="info-row">
    <span class="info-label">Referer:</span>
    <span class="info-value">${req.headers.get('referer') || '-'}</span>
  </div>
  <div class="info-row">
    <span class="info-label">File:</span>
    <span class="info-value">${filepath}</span>
  </div>

  <div class="error-msg">${errorMessage}</div>

  <div class="stack">${stackTrace}</div>
</body>
</html>
      `.trim();
      return new Response(html, {
        status: 500,
        headers: { "Content-Type": "text/html; charset=utf-8" },
      });
    } else {
      // Production mode: show generic error message
      const html = `
<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body>
  <h1>500 Internal Server Error</h1>
  <p>An internal server error occurred.</p>
</body>
</html>
      `.trim();
      return new Response(html, {
        status: 500,
        headers: { "Content-Type": "text/html; charset=utf-8" },
      });
    }
  }
}

// Start server
async function main(): Promise<void> {
  // Create temporary logger for config loading phase
  const tempLogger = createDefaultLogger();

  const config = await parseArgs(tempLogger);

  // Resolve root directory to absolute path (same logic as config reload)
  const rootPath = config.root;
  if (configFilepath) {
    const configDir = dirname(configFilepath);
    // If root is relative path, resolve relative to config file directory
    if (!rootPath.startsWith("/") && !rootPath.match(/^[a-zA-Z]:/)) {
      config.root = resolve(configDir, rootPath);
    } else {
      config.root = resolve(rootPath);
    }
  } else {
    config.root = resolve(rootPath);
  }

  // Register dependency injection function (type-safe version)
  registerDep("testFunc", () => {
    return function testFunc() {
      serverLogger.debug("testFunc called");
      return "testFunc called";
    };
  });

  // Register cookie manager
  registerDep("cookies", (ctx) => {
    return createCookieManager(ctx);
  });

  // Global SessionStore singleton
  let sessionStore: SessionStoreInterface | null = null;

  // Create logger for session cleanup (needs to be before session registration)
  const loggerConfig = config.logger;
  let loggerInstance: Logger | undefined;
  if (loggerConfig) {
    loggerInstance = createProductionLogger(loggerConfig);
  } else if (config.dev) {
    loggerInstance = createDefaultLogger();
  } else {
    loggerInstance = createProductionLogger();
  }

  // Register session dependency (now synchronous!)
  registerDep("session", (ctx) => {
    // Initialize global store (only once)
    if (!sessionStore) {
      // Read secret from environment variable
      const secret = Deno.env.get("TSP_SESSION_SECRET");
      const secretBytes = secret ? new TextEncoder().encode(secret) : undefined; // Auto-generated in dev mode

      // Read session options from config
      const sessionConfig = config.session;

      const options = {
        ...getDefaultOptions(),
        secret: secretBytes,
        // Override options read from config
        ...(sessionConfig?.secure !== undefined &&
          { secure: sessionConfig.secure }),
        ...(sessionConfig?.httpOnly !== undefined &&
          { httpOnly: sessionConfig.httpOnly }),
        ...(sessionConfig?.sameSite !== undefined &&
          { sameSite: sessionConfig.sameSite }),
        ...(sessionConfig?.path !== undefined && { path: sessionConfig.path }),
        ...(sessionConfig?.maxAge !== undefined &&
          { maxAge: sessionConfig.maxAge }),
        ...(sessionConfig?.cookieName !== undefined &&
          { cookieName: sessionConfig.cookieName }),
        ...(sessionConfig?.rolling !== undefined &&
          { rolling: sessionConfig.rolling }),
      };

      // Check if Redis is configured for session sharing
      const redisConfig = config.redis;
      if (redisConfig && redisConfig.host) {
        sessionStore = new RedisSessionStore(
          options,
          {
            host: redisConfig.host,
            port: redisConfig.port || 6379,
            password: redisConfig.password,
            db: redisConfig.db,
          },
          loggerInstance,
        );
        console.log("✓ Using Redis session store");
        loggerInstance?.info("Using Redis session store", {
          host: redisConfig.host,
          port: redisConfig.port,
        });
      } else {
        // Use in-memory session store
        sessionStore = new SessionStore(options, loggerInstance);
      }
    }

    // Get cookie manager for setting cookies
    const cookieManager = createCookieManager(ctx);

    // Create session manager
    return createSessionManager(ctx, sessionStore, cookieManager);
  });

  // Register Response helper
  registerDep("response", (ctx) => {
    return createResponseHelper(ctx);
  });

  // Register Logger (singleton) - reuse loggerInstance created earlier
  // serverLogger is a reference to loggerInstance for global logging
  const serverLogger = loggerInstance!;

  registerDep("logger", () => {
    return loggerInstance!;
  });

  // Register createMySQL factory function
  registerDep("createMySQL", (ctx: PageContext) => {
    return async (config: globalThis.MySQLConfig, zod?: any) => {
      // If zod not passed, get from dependencies
      const z = zod || (ctx as any).deps?.z || (globalThis as any).z;
      return createMySQL(config, z);
    };
  });

  // Register createRedis factory function
  registerDep("createRedis", () => {
    return createRedis;
  });

  // Register createLdap factory function
  registerDep("createLdap", () => {
    return createLdapClient;
  });

  // Register createExcelJS factory function
  registerDep("createExcelJS", () => {
    return createExcelJSImport;
  });

  // Register Crypto helper (native Deno API)
  registerDep("crypto", () => {
    const subtle = crypto.subtle as any;
    return {
      // Generate random values
      getRandomValues: (length: number) => crypto.getRandomValues(new Uint8Array(length)),

      // Hash data
      digest: (algo: string, data: string | Uint8Array) => {
        const encoded = typeof data === "string" ? new TextEncoder().encode(data) : data;
        return subtle.digest(algo, encoded);
      },

      // Generate key
      generateKey: (
        algo: "AES-GCM" | "HMAC",
        length?: number,
        extractable = false,
        usages?: ("encrypt" | "decrypt" | "sign" | "verify")[],
      ) => {
        if (algo === "AES-GCM") {
          return subtle.generateKey(
            { name: "AES-GCM", length: length || 256 },
            extractable,
            usages?.length ? usages : ["encrypt", "decrypt"],
          );
        } else if (algo === "HMAC") {
          return subtle.generateKey(
            { name: "HMAC", hash: "SHA-256" },
            extractable,
            usages?.length ? usages : ["sign", "verify"],
          );
        }
        throw new Error(`Unsupported algorithm: ${algo}`);
      },

      // Import key
      importKey: (
        algo: "AES-GCM" | "HMAC",
        keyData: string | Uint8Array,
        extractable = false,
        usages?: ("encrypt" | "decrypt" | "sign" | "verify")[],
      ) => {
        const encoded = typeof keyData === "string" ? new TextEncoder().encode(keyData) : keyData;

        if (algo === "AES-GCM") {
          return subtle.importKey("raw", encoded, { name: "AES-GCM" }, extractable, usages?.length ? usages : ["encrypt", "decrypt"]);
        } else if (algo === "HMAC") {
          return subtle.importKey("raw", encoded, { name: "HMAC", hash: "SHA-256" }, extractable, usages?.length ? usages : ["sign", "verify"]);
        }
        throw new Error(`Unsupported algorithm: ${algo}`);
      },

      // AES-GCM encrypt
      encrypt: (data: Uint8Array, key: CryptoKey, iv: Uint8Array) => subtle.encrypt({ name: "AES-GCM", iv }, key, data),

      // AES-GCM decrypt
      decrypt: (data: Uint8Array, key: CryptoKey, iv: Uint8Array) => subtle.decrypt({ name: "AES-GCM", iv }, key, data),

      // HMAC sign
      sign: (algo: string, key: CryptoKey, data: Uint8Array) => subtle.sign({ name: algo }, key, data),

      // HMAC verify
      verify: (algo: string, key: CryptoKey, signature: Uint8Array, data: Uint8Array) =>
        subtle.verify({ name: algo }, key, signature, data),
    };
  });

  // Register createBcryptjs factory function
  registerDep("createBcryptjs", () => {
    return async (config?: { saltRounds?: number }) => {
      const saltRounds = config?.saltRounds || 10;

      // Load bcryptjs from esm.sh
      const mod = await import("https://esm.sh/bcryptjs@3.0.3");
      const bcryptjs = mod.default || mod;

      return {
        hash: (password: string) => bcryptjs.hashSync(password, saltRounds),
        compare: (password: string, hash: string) => bcryptjs.compareSync(password, hash),
      };
    };
  });

  // Register TSP Info
  const { TspInfo } = await import("./tspinfo.ts");
  registerDep("tspinfo", () => {
    return new TspInfo(
      TSP_VERSION,
      {
        root: config.root,
        port: config.port,
        dev: config.dev,
        accessLog: config.accessLog,
        logger: config.logger,
      },
      serverLogger,
    );
  });

  // Register test helper
  const { createTestHelper } = await import("./test_helper.ts");
  registerDep("testHelper", () => {
    return createTestHelper();
  });

  // Log server startup information
  serverLogger.info("TSP Server starting", {
    root: config.root,
    port: config.port,
    mode: config.dev ? "Development" : "Production",
  });

  // Print startup banner (kept for user-friendly console output)
  const banner = `
╔════════════════════════════════════════╗
║            TSP Server                  ║
╚════════════════════════════════════════╝

Version: ${TSP_VERSION}
Document Root: ${config.root}
Port: ${config.port}
Mode: ${config.dev ? "Development" : "Production"}

Starting server...
  `;
  console.log(banner);
  serverLogger.debug("Server config", {
    root: config.root,
    port: config.port,
    mode: config.dev ? "Development" : "Production",
  });

  // File manager config hint
  if (config.fileManager?.enabled) {
    const fmMsg = `
✓ File manager enabled
  Access path: ${config.fileManager.path || "/__filemanager"}
  ✓ Config file changes will auto-reload
    `;
    console.log(fmMsg);
    serverLogger.info("File manager enabled", {
      path: config.fileManager.path || "/__filemanager",
      note: "Config file changes will auto-reload",
    });
  }

  // Print config file info
  if (configFilepath) {
    console.log(`✓ Using config file: ${configFilepath}`);
  }

  // Start HTTP server
  Deno.serve({
    port: config.port,
    onListen: ({ port, hostname }) => {
      const serverUrl = `http://${hostname}:${port}/`;
      console.log(`✓ Server running at ${serverUrl}`);
      console.log("Press Ctrl+C to stop.\n");
      serverLogger.info("Server started successfully", {
        url: serverUrl,
        port,
        hostname,
      });
    },
  }, async (req) => {
    const resp = await handleRequest(req, config, serverLogger);
    // Log access log
    await logAccess(req, resp, config, serverLogger);
    return resp;
  });
}

// Start program
if (import.meta.main) {
  main().catch((error) => {
    // Create temporary logger for logging startup errors
    const tempLogger = createDefaultLogger();
    tempLogger.error("Fatal error:", error);
    console.error("Fatal error:", error);
    Deno.exit(1);
  });
}
