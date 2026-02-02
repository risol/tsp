#!/usr/bin/env -S deno run --allow-net --allow-read

/**
 * TSP: TypeScript Server Page
 * 使用 Deno + TSX 实现的模板服务器
 */

import { resolvePath, securityCheck } from "./router.ts";
import { buildContext } from "./context.ts";
import { getPage, type RedirectResult, renderJSX } from "./cache.ts";
import { registerDep } from "./injection-typed.ts";
import { compileAll, setCacheBaseDir } from "./precompiler_lib.ts";
import { serveStaticFileWithCache } from "./static.ts";
import { join, relative, resolve, dirname } from "std/path";
import { createMySQL } from "./mysql/factory.ts";
import { createRedis } from "./redis/factory.ts";
import { createLdapClient } from "./ldap/client.ts";
import {
  createSessionManager,
  getDefaultOptions,
  SessionStore,
} from "./session.ts";
import { createCookieManager } from "./cookies.ts";
import { createResponseHelper } from "./response.ts";
import { parseMultipartFormData, type UploadedFile } from "./files.ts";
import {
  createDefaultLogger,
  createProductionLogger,
  type Logger,
} from "./logger.ts";
import { nanoid } from "nanoid";
import type { FileManagerConfig } from "./filemanager/types.ts";
import { validateFileManagerConfig } from "./filemanager/config.ts";

// 日志配置接口
export interface LoggerConfig {
  /** 最小日志级别：DEBUG, INFO, WARN, ERROR */
  level?: "DEBUG" | "INFO" | "WARN" | "ERROR";
  /** 日志文件路径（可选） */
  file?: string;
  /** 是否启用彩色输出 */
  colorize?: boolean;
  /** 日志格式：text 或 json */
  format?: "text" | "json";
  /** 日志归档配置 */
  rotation?: {
    /** 单个日志文件最大大小（字节），默认 10MB */
    maxSize?: number;
    /** 保留的归档文件数量，默认 5 */
    maxFiles?: number;
    /** 是否压缩归档文件（gzip），默认 false */
    compress?: boolean;
    /** 按日期归档：每天创建新文件 */
    daily?: boolean;
  };
}

// 配置接口
export interface Config {
  root: string;
  port: number;
  dev: boolean;
  accessLogPath?: string;
  staticExtensions?: string[];
  /** 日志配置 */
  logger?: LoggerConfig;
  /** 文件管理器配置 */
  fileManager?: FileManagerConfig;
}

// 默认支持的静态文件扩展名
const DEFAULT_STATIC_EXTENSIONS = [
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

// 默认配置
const DEFAULT_CONFIG: Config = {
  root: "./www",
  port: 9000,
  dev: false,
  staticExtensions: DEFAULT_STATIC_EXTENSIONS,
};

// 配置文件接口（与 Config 相同，但所有字段都是可选的）
interface ConfigFile {
  root?: string;
  port?: number;
  dev?: boolean;
  accessLogPath?: string;
  staticExtensions?: string[];
  /** 日志配置 */
  logger?: LoggerConfig;
  /** 文件管理器配置 */
  fileManager?: FileManagerConfig;
}

/**
 * 移除 JSONC 注释
 * @param content JSONC 内容
 * @returns 纯 JSON 内容
 */
function stripJsonComments(content: string): string {
  // 移除单行注释 //
  content = content.replace(/\/\/.*$/gm, "");
  // 移除多行注释 /* */
  content = content.replace(/\/\*[\s\S]*?\*\//g, "");
  return content;
}

/**
 * 记录 HTTP 访问日志
 * @param req 请求对象
 * @param resp 响应对象
 * @param config 配置对象
 * @param logger 可选的 logger 实例
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

  // 获取响应时间（如果需要的话，可以在这里添加计时逻辑）
  const logLine = `${timestamp} ${method} ${pathname} ${status} "${userAgent}"`;

  if (config.accessLogPath) {
    // 写入文件
    try {
      // 确保日志目录存在
      const logDir = dirname(config.accessLogPath);
      await Deno.mkdir(logDir, { recursive: true });

      await Deno.writeTextFile(
        config.accessLogPath,
        logLine + "\n",
        { append: true, create: true },
      );
    } catch (error) {
      logger?.error("写入访问日志失败", error);
    }
  } else {
    // 输出到控制台
    console.log(logLine);
  }
}

/**
 * 从配置文件加载配置
 * @param filepath 配置文件路径
 * @param logger 可选的 logger 实例
 * @returns 配置对象
 */
async function loadConfigFile(
  filepath: string,
  logger?: Logger,
): Promise<Config> {
  try {
    let content = await Deno.readTextFile(filepath);

    // 如果是 JSONC 文件，移除注释
    if (filepath.endsWith(".jsonc")) {
      content = stripJsonComments(content);
    }

    const config: ConfigFile = JSON.parse(content);

    // 验证文件管理器配置（如果存在）
    if (config.fileManager) {
      logger?.info("发现文件管理器配置", { enabled: config.fileManager.enabled });
      const validated = validateFileManagerConfig(config.fileManager);
      logger?.info("文件管理器配置验证完成", {
        enabled: validated.enabled,
        path: validated.path,
      });
      // 如果未启用，移除配置
      if (!validated.enabled) {
        logger?.info("文件管理器未启用，移除配置");
        delete config.fileManager;
      } else {
        // 替换为验证后的配置
        logger?.info("文件管理器已启用并验证通过");
        (config as Config).fileManager = validated;
      }
    } else {
      logger?.info("未配置文件管理器");
    }

    // 合并默认配置和文件配置
    return {
      ...DEFAULT_CONFIG,
      ...config,
    };
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      const msg = `配置文件不存在: ${filepath}`;
      logger?.error(msg);
      console.error(msg);
      Deno.exit(1);
    } else if (error instanceof SyntaxError) {
      const msg = `配置文件格式错误: ${error.message}`;
      logger?.error(msg);
      console.error(msg);
      Deno.exit(1);
    } else {
      throw error;
    }
  }
}

// 解析命令行参数
async function parseArgs(logger?: Logger): Promise<Config> {
  const args = Deno.args;
  let config = { ...DEFAULT_CONFIG };
  let configFile: string | null = null;

  // 第一遍解析：查找 --config 参数
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "--config" || arg === "-c") {
      configFile = args[++i];
      break;
    }
  }

  // 如果指定了配置文件，先加载配置文件
  if (configFile) {
    config = await loadConfigFile(configFile, logger);
  } else {
    // 尝试自动查找默认配置文件
    const defaultConfigFiles = [
      "config.jsonc",
    ];

    for (const filename of defaultConfigFiles) {
      try {
        await Deno.stat(filename);
        const msg = `✓ 找到配置文件: ${filename}`;
        logger?.info(msg);
        console.log(msg);
        config = await loadConfigFile(filename, logger);
        break;
      } catch {
        // 文件不存在，继续查找
      }
    }
  }

  // 第二遍解析：命令行参数覆盖配置文件（优先级更高）
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
      case "--config":
      case "-c":
        // 跳过配置文件路径（已在第一遍处理）
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
        config.accessLogPath = args[++i];
        break;
      case "--help":
      case "-h":
        printHelp();
        Deno.exit(0);
      default:
        // 忽略未知参数，继续处理
        break;
    }
  }

  return config;
}

// 打印帮助信息
function printHelp(): void {
  console.log(`
TSP: TypeScript Server Page

用法:
  ./tspserver [options]

选项:
  --config, -c <file>     配置文件路径 (默认: config.jsonc)
  --root, -r <path>       文档根目录 (默认: ./www)
  --port, -p <port>       监听端口 (默认: 9000)
  --dev, -d               开发模式 (显示错误详情)
  --access-log, -a <path> 访问日志文件路径 (默认: 控制台输出)
  --help, -h              显示帮助信息

配置文件:
  支持的配置文件名: config.jsonc

  配置文件格式 (JSONC，支持注释):
  {
    "root": "./www",
    "port": 9000,
    "dev": false,
    "accessLogPath": "./access.log",
    "staticExtensions": [".css", ".js", ".png", ".jpg"]
  }

  优先级: 命令行参数 > 配置文件 > 默认值

示例:
  # 使用配置文件
  ./tspserver

  # 指定配置文件
  ./tspserver --config ./my-config.jsonc

  # 命令行参数覆盖配置文件
  ./tspserver --port 8080 --dev

  # 纯命令行参数
  ./tspserver --root ./www --port 9000 --dev

  # 记录访问日志到文件
  ./tspserver --access-log ./access.log

  # 访问日志输出到控制台（默认）
  ./tspserver
`);
}

// 处理请求
async function handleRequest(
  req: Request,
  config: Config,
  serverLogger: Logger,
): Promise<Response> {
  try {
    const url = new URL(req.url);
    const pathname = url.pathname;

    // 文件管理器路由拦截
    if (config.fileManager?.enabled) {
      serverLogger.debug("文件管理器已启用", {
        path: config.fileManager.path,
        enabled: config.fileManager.enabled,
      });

      const fmPath = config.fileManager.path || "/__filemanager";
      // 确保路径比较时格式一致
      const normalizedFmPath = fmPath.endsWith("/") ? fmPath.slice(0, -1) : fmPath;
      const normalizedPathname = pathname.endsWith("/") && pathname !== "/"
        ? pathname.slice(0, -1)
        : pathname;

      serverLogger.debug("路径匹配检查", {
        pathname,
        normalizedPathname,
        normalizedFmPath,
        startsWith: normalizedPathname.startsWith(normalizedFmPath + "/"),
      });

      if (normalizedPathname === normalizedFmPath || normalizedPathname.startsWith(normalizedFmPath + "/")) {
        serverLogger.info("文件管理器路由匹配", { pathname });
        const { handleFileManagerRequest } = await import("./filemanager/mod.ts");
        // 确保传递完整验证过的配置
        const fmConfig = validateFileManagerConfig(config.fileManager);
        return handleFileManagerRequest(req, fmConfig, config.root, serverLogger);
      }
    }

    // 解析文件路径（包含静态文件扩展名）
    const staticExtensions = config.staticExtensions || [];
    const fileResult = resolvePath(pathname, config.root, staticExtensions);
    if (!fileResult.success) {
      return new Response(fileResult.error, {
        status: 404,
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }

    const filepath = fileResult.filepath!;

    // 安全检查（包含静态文件扩展名）
    const securityResult = await securityCheck(
      filepath,
      config.root,
      staticExtensions,
    );
    if (!securityResult.success) {
      // 根据错误类型决定状态码
      const error = securityResult.error || "";

      // 文件不存在 → 404
      // 权限拒绝 → 403
      // 其他错误 → 500
      let status = 500;
      if (
        error.includes("File not found") || error.includes("not found") ||
        error.includes("Directory index")
      ) {
        status = 404;
      } else if (
        error.includes("Access denied") ||
        error.includes("File type not allowed")
      ) {
        status = 403;
      }

      return new Response(error, {
        status,
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }

    // 检查是否是静态文件（如果是配置文件中允许的扩展名）
    // 静态文件扩展名列表在配置中提供
    const allowedExtensions = config.staticExtensions || [];
    const staticResponse = await serveStaticFileWithCache(
      filepath,
      allowedExtensions,
      req.headers,
      config.dev,
    );

    // 如果是静态文件，直接返回
    if (staticResponse !== null) {
      return staticResponse;
    }

    // 解析请求体
    let body: unknown = null;
    // 解析上传的文件
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
        // 解析文件上传
        try {
          const arrayBuffer = await req.arrayBuffer();
          const requestBody = new Uint8Array(arrayBuffer);
          const result = await parseMultipartFormData(
            requestBody,
            contentType,
            {
              maxSize: 10 * 1024 * 1024, // 10MB 默认限制
            },
          );
          body = result.fields;
          files = result.files;
        } catch (error) {
          serverLogger.error("解析 multipart 表单数据失败", error);
          body = null;
          files = {};
        }
      } else {
        body = await req.text();
      }
    }

    // 解析 cookies
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

    // 解析查询参数
    const query: Record<string, string> = {};
    url.searchParams.forEach((value, key) => {
      query[key] = value;
    });

    // 构建上下文
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

    // 获取并执行页面函数
    const pageFn = await getPage(filepath);
    const result = await pageFn(context);

    // 提取cookie响应头
    const { extractSetCookieHeaders } = await import("./cookies.ts");
    const setCookieHeaders = extractSetCookieHeaders(context);

    // 检查是否是重定向对象
    if (result && typeof result === "object" && "redirect" in result) {
      // 更严格地检查：确保不是 VNode
      const isVNode = "type" in result || "props" in result ||
        "__k" in result || "__" in result;

      if (!isVNode) {
        const redirectResult = result as RedirectResult;
        const targetUrl = redirectResult.redirect;
        const status = redirectResult.status ?? 302;

        // 验证重定向状态码
        const validStatuses = [301, 302, 303, 307, 308];
        const finalStatus = validStatuses.includes(status) ? status : 302;

        const headers: HeadersInit = { "Location": targetUrl };

        // 添加Set-Cookie响应头
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

    // 检查是否是 Response 对象（直接返回）
    if (result instanceof Response) {
      // 如果有cookie需要设置，创建新的Response对象并添加Set-Cookie头
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

    // 默认：渲染 JSX 为 HTML
    const html = renderJSX(result);

    // 设置响应头
    const headers: HeadersInit = {
      "Content-Type": "text/html; charset=utf-8",
    };

    // 开发模式：禁用浏览器缓存
    if (config.dev) {
      headers["Cache-Control"] = "no-cache, no-store, must-revalidate";
      headers["Pragma"] = "no-cache";
      headers["Expires"] = "0";
    }

    // 添加Set-Cookie响应头
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
    // 错误处理
    const errorMessage = error instanceof Error ? error.message : String(error);
    const stackTrace = error instanceof Error ? error.stack : "";

    // 使用 logger 记录错误
    serverLogger.error("请求处理错误", {
      error: errorMessage,
      stack: stackTrace,
      url: req.url,
      method: req.method,
    });

    if (config.dev) {
      // 开发模式：显示详细错误
      const html = `
<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body>
  <h1>500 Internal Server Error</h1>
  <pre>${errorMessage}</pre>
  <pre>${stackTrace}</pre>
</body>
</html>
      `.trim();
      return new Response(html, {
        status: 500,
        headers: { "Content-Type": "text/html; charset=utf-8" },
      });
    } else {
      // 生产模式：隐藏错误详情
      const html = `
<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body>
  <h1>500 Internal Server Error</h1>
  <p>An error occurred while processing your request.</p>
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

// 启动服务器
async function main(): Promise<void> {
  // 创建临时 logger 用于配置加载阶段
  const tempLogger = createDefaultLogger();

  const config = await parseArgs(tempLogger);

  // 将根目录解析为绝对路径
  config.root = resolve(config.root);

  // 设置缓存基础目录为当前工作目录
  // 这确保了缓存相对于运行二进制文件的目录创建
  // 无论是从项目根目录还是 dist/ 运行，缓存都会在 ./cache/tsp/ 中
  setCacheBaseDir(Deno.cwd());

  // 注册依赖注入函数（类型安全版本）
  registerDep("testFunc", () => {
    return function testFunc() {
      serverLogger.debug("testFunc called");
      return "testFunc called";
    };
  });

  // 注册 cookie 管理器
  registerDep("cookies", (ctx) => {
    return createCookieManager(ctx);
  });

  // 全局 SessionStore 单例
  let sessionStore: SessionStore | null = null;

  // 注册 session 依赖（现在是同步的！）
  registerDep("session", (ctx) => {
    // 初始化全局 store（仅一次）
    if (!sessionStore) {
      // 从环境变量读取密钥
      const secret = Deno.env.get("TSP_SESSION_SECRET");
      const secretBytes = secret ? new TextEncoder().encode(secret) : undefined; // 开发环境会自动生成

      const options = {
        ...getDefaultOptions(),
        secret: secretBytes,
      };

      sessionStore = new SessionStore(options);
    }

    // 获取用于设置 cookies 的 cookie 管理器
    const cookieManager = createCookieManager(ctx);

    // 创建 session 管理器
    return createSessionManager(ctx, sessionStore, cookieManager);
  });

  // 注册 Response 辅助器
  registerDep("response", (ctx) => {
    return createResponseHelper(ctx);
  });

  // 注册 Logger（单例）
  let loggerInstance: Logger | null = null;

  // 创建 logger 实例（在 registerDep 之前，确保 serverLogger 能使用）
  const loggerConfig = config.logger;
  if (loggerConfig) {
    // 使用配置文件创建 logger（包含归档配置）
    loggerInstance = createProductionLogger(loggerConfig);
  } else if (config.dev) {
    // 开发模式默认日志
    loggerInstance = createDefaultLogger();
  } else {
    // 生产模式默认日志（无文件输出）
    loggerInstance = createProductionLogger();
  }

  registerDep("logger", () => {
    return loggerInstance!;
  });

  // 创建服务器级 logger 实例（用于服务器启动、错误等系统日志）
  const serverLogger = loggerInstance;

  // 设置缓存模块和预编译模块的 logger
  const { setCacheLogger } = await import("./cache.ts");
  const { setPrecompilerLogger } = await import("./precompiler_lib.ts");
  setCacheLogger(serverLogger);
  setPrecompilerLogger(serverLogger);

  // 注册 nanoid
  registerDep("nanoid", () => nanoid);

  // 注册 createMySQL 工厂函数
  registerDep("createMySQL", () => {
    return createMySQL;
  });

  // 注册 createRedis 工厂函数
  registerDep("createRedis", () => {
    return createRedis;
  });

  // 注册 createLdap 工厂函数
  registerDep("createLdap", () => {
    return createLdapClient;
  });

  // 注册 TSP Info
  const { TspInfo } = await import("./tspinfo.ts");
  registerDep("tspinfo", () => {
    return new TspInfo(
      {
        root: config.root,
        port: config.port,
        dev: config.dev,
        accessLogPath: config.accessLogPath,
        logger: config.logger,
      },
      serverLogger,
    );
  });

  // 记录服务器启动信息
  serverLogger.info("TSP Server 启动中", {
    root: config.root,
    port: config.port,
    mode: config.dev ? "Development" : "Production",
  });

  // 打印启动 banner（保留用于用户友好的控制台输出）
  const banner = `
╔════════════════════════════════════════╗
║            TSP Server                  ║
╚════════════════════════════════════════╝

Document Root: ${config.root}
Port: ${config.port}
Mode: ${config.dev ? "Development" : "Production"}

Starting server...
  `;
  console.log(banner);
  serverLogger.debug("服务器配置", {
    root: config.root,
    port: config.port,
    mode: config.dev ? "Development" : "Production",
  });

  // 预编译所有 TSX 文件
  if (config.dev) {
    const devMsg = "\n⏭️  Development mode: skipping precompilation\n   Files will be compiled on-demand.\n";
    console.log(devMsg);
    serverLogger.info("开发模式：跳过预编译");
  } else {
    const precompileMsg = "\n🔨 Precompiling TSX files...";
    console.log(precompileMsg);
    serverLogger.info("开始预编译 TSX 文件");
    const { compileAll } = await import("./precompiler_lib.ts");

    // 计算相对于 CWD 的根目录路径
    const rootDir = relative(Deno.cwd(), config.root);
    const compiledFiles = await compileAll(rootDir);

    if (compiledFiles.length === 0) {
      const noCompileMsg = "⚠️  No files were successfully precompiled";
      console.warn(noCompileMsg);
      serverLogger.warn("没有文件成功预编译");
    } else {
      serverLogger.info("预编译完成", { count: compiledFiles.length });

      // 预热缓存：加载所有编译后的模块到内存
      const warmupMsg = "🔥 Warming up cache...";
      console.log(warmupMsg);
      serverLogger.info("开始预热缓存");
      const { getPage } = await import("./cache.ts");

      for (const relPath of compiledFiles) {
        try {
          // 转换为 router 返回的格式：./www/index.tsx
          const cacheKey = join(".", relPath);
          await getPage(cacheKey);
        } catch (error) {
          const err = error as Error;
          const warnMsg = `[WARN] Failed to load ${relPath} into cache: ${err.message}`;
          console.warn(warnMsg);
          serverLogger.warn("缓存加载失败", {
            file: relPath,
            error: err.message,
          });
        }
      }

      const warmupDoneMsg = "✓ Cache warmed up";
      console.log(warmupDoneMsg);
      serverLogger.info("缓存预热完成", { count: compiledFiles.length });
    }
  }

  // 启动 HTTP 服务器
  Deno.serve({
    port: config.port,
    onListen: ({ port, hostname }) => {
      const serverUrl = `http://${hostname}:${port}/`;
      console.log(`✓ Server running at ${serverUrl}`);
      console.log("按 Ctrl+C 停止。\n");
      serverLogger.info("服务器启动成功", {
        url: serverUrl,
        port,
        hostname,
      });
    },
  }, async (req) => {
    const resp = await handleRequest(req, config, serverLogger);
    // 记录访问日志
    await logAccess(req, resp, config, serverLogger);
    return resp;
  });
}

// 启动程序
if (import.meta.main) {
  main().catch((error) => {
    // 创建临时 logger 用于记录启动错误
    const tempLogger = createDefaultLogger();
    tempLogger.error("Fatal error:", error);
    console.error("Fatal error:", error);
    Deno.exit(1);
  });
}
