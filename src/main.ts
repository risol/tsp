#!/usr/bin/env -S deno run --allow-net --allow-read

/**
 * TSP: TypeScript Server Page
 * 使用 Deno + TSX 实现的模板服务器
 */

import { resolvePath, securityCheck } from "./router.ts";
import { buildContext } from "./context.ts";
import { getPage, renderJSX, type RedirectResult } from "./cache.ts";
import { registerDep } from "./injection-typed.ts";
import { compileAll, setCacheBaseDir } from "./precompiler_lib.ts";
import { serveStaticFileWithCache } from "./static.ts";
import { join, resolve } from "std/path";

// 配置接口
export interface Config {
  root: string;
  port: number;
  dev: boolean;
  accessLogPath?: string;
  staticExtensions?: string[];
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
}

/**
 * 移除 JSONC 注释
 * @param content JSONC 内容
 * @returns 纯 JSON 内容
 */
function stripJsonComments(content: string): string {
  // 移除单行注释 //
  content = content.replace(/\/\/.*$/gm, '');
  // 移除多行注释 /* */
  content = content.replace(/\/\*[\s\S]*?\*\//g, '');
  return content;
}

/**
 * 记录 HTTP 访问日志
 * @param req 请求对象
 * @param resp 响应对象
 * @param config 配置对象
 */
export async function logAccess(
  req: Request,
  resp: Response,
  config: Config
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
      await Deno.writeTextFile(
        config.accessLogPath,
        logLine + "\n",
        { append: true, create: true }
      );
    } catch (error) {
      console.error(`Failed to write access log: ${error}`);
    }
  } else {
    // 输出到控制台
    console.log(logLine);
  }
}

/**
 * 从配置文件加载配置
 * @param filepath 配置文件路径
 * @returns 配置对象
 */
async function loadConfigFile(filepath: string): Promise<Config> {
  try {
    let content = await Deno.readTextFile(filepath);

    // 如果是 JSONC 文件，移除注释
    if (filepath.endsWith('.jsonc')) {
      content = stripJsonComments(content);
    }

    const config: ConfigFile = JSON.parse(content);

    // 合并默认配置和文件配置
    return {
      ...DEFAULT_CONFIG,
      ...config,
    };
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      console.error(`配置文件不存在: ${filepath}`);
      Deno.exit(1);
    } else if (error instanceof SyntaxError) {
      console.error(`配置文件格式错误: ${error.message}`);
      Deno.exit(1);
    } else {
      throw error;
    }
  }
}

// 解析命令行参数
async function parseArgs(): Promise<Config> {
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
    config = await loadConfigFile(configFile);
  } else {
    // 尝试自动查找默认配置文件
    const defaultConfigFiles = [
      "config.jsonc",
      "config.json",
    ];

    for (const filename of defaultConfigFiles) {
      try {
        await Deno.stat(filename);
        console.log(`✓ 找到配置文件: ${filename}`);
        config = await loadConfigFile(filename);
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
  --config, -c <file>     配置文件路径 (默认: 自动查找 config.json)
  --root, -r <path>       文档根目录 (默认: ./www)
  --port, -p <port>       监听端口 (默认: 9000)
  --dev, -d               开发模式 (显示错误详情)
  --access-log, -a <path> 访问日志文件路径 (默认: 控制台输出)
  --help, -h              显示帮助信息

配置文件:
  支持的配置文件名（按优先级）:
    - config.jsonc
    - config.json

  配置文件格式 (JSON):
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
  ./tspserver --config ./my-config.json

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
  config: Config
): Promise<Response> {
  try {
    const url = new URL(req.url);
    const pathname = url.pathname;

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
    const securityResult = await securityCheck(filepath, config.root, staticExtensions);
    if (!securityResult.success) {
      // 根据错误类型决定状态码
      const error = securityResult.error || "";

      // 文件不存在 → 404
      // 权限拒绝 → 403
      // 其他错误 → 500
      let status = 500;
      if (error.includes("File not found") || error.includes("not found") || error.includes("Directory index")) {
        status = 404;
      } else if (error.includes("Access denied") || error.includes("File type not allowed")) {
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
      config.dev
    );

    // 如果是静态文件，直接返回
    if (staticResponse !== null) {
      return staticResponse;
    }

    // 解析请求体
    let body: unknown = null;
    const contentType = req.headers.get("content-type") || "";
    if (req.method === "POST" || req.method === "PUT" || req.method === "PATCH") {
      if (contentType.includes("application/json")) {
        try {
          body = await req.json();
        } catch {
          body = null;
        }
      } else if (contentType.includes("application/x-www-form-urlencoded")) {
        const text = await req.text();
        body = Object.fromEntries(new URLSearchParams(text));
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
      method: req.method as "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS",
      url,
      headers: req.headers,
      query,
      body,
      cookies,
      file: filepath,
      root: config.root,
    });

    // 获取并执行页面函数
    const pageFn = await getPage(filepath);
    const result = await pageFn(context);

    // 检查是否是重定向对象
    if (result && typeof result === "object" && "redirect" in result) {
      // 更严格地检查：确保不是 VNode
      const isVNode = "type" in result || "props" in result || "__k" in result || "__" in result;

      if (!isVNode) {
        const redirectResult = result as RedirectResult;
        const targetUrl = redirectResult.redirect;
        const status = redirectResult.status ?? 302;

        // 验证重定向状态码
        const validStatuses = [301, 302, 303, 307, 308];
        const finalStatus = validStatuses.includes(status) ? status : 302;

        return new Response(null, {
          status: finalStatus,
          headers: {
            "Location": targetUrl,
          },
        });
      }
    }

    // 检查是否是 Response 对象（直接返回）
    if (result instanceof Response) {
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

    return new Response(html, {
      status: 200,
      headers,
    });
  } catch (error) {
    // 错误处理
    const errorMessage = error instanceof Error ? error.message : String(error);
    const stackTrace = error instanceof Error ? error.stack : "";

    console.error("Request error:", errorMessage);

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
  const config = await parseArgs();

  // Resolve root directory to absolute path
  config.root = resolve(config.root);

  // Set cache base directory to current working directory
  // This ensures cache is created relative to where the binary is run from
  // Whether running from project root or dist/, the cache will be in ./cache/tsp/
  setCacheBaseDir(Deno.cwd());

  // 注册依赖注入函数（类型安全版本）
  registerDep('testFunc', () => {
    return function testFunc() {
      console.log('testFunc called');
      return 'testFunc called';
    };
  });

  console.log(`
╔════════════════════════════════════════╗
║            TSP Server                  ║
╚════════════════════════════════════════╝

Document Root: ${config.root}
Port: ${config.port}
Mode: ${config.dev ? "Development" : "Production"}

Starting server...
  `);

  // 预编译所有 TSX 文件
  if (config.dev) {
    console.log("\n⏭️  Development mode: skipping precompilation");
    console.log("   Files will be compiled on-demand.\n");
  } else {
    console.log("\n🔨 Precompiling TSX files...");
    const { compileAll } = await import("./precompiler_lib.ts");
    try {
      const compiledFiles = await compileAll();

      // 预热缓存：加载所有编译后的模块到内存
      console.log("🔥 Warming up cache...");
      const { getPage } = await import("./cache.ts");

      for (const relPath of compiledFiles) {
        try {
          // 转换为 router 返回的格式：./www/index.tsx
          const cacheKey = join(".", relPath);
          await getPage(cacheKey);
        } catch (error) {
          const err = error as Error;
          console.warn(`[WARN] Failed to load ${relPath} into cache:`, err.message);
        }
      }

      console.log("✓ Cache warmed up");
    } catch (error) {
      const err = error as Error;
      console.error("\n❌ Precompilation failed:", err.message);
      Deno.exit(1);
    }
  }

  // 启动 HTTP 服务器
  Deno.serve({
    port: config.port,
    onListen: ({ port, hostname }) => {
      console.log(`✓ Server running at http://${hostname}:${port}/`);
      console.log("Press Ctrl+C to stop.\n");
    },
  }, async (req) => {
    const resp = await handleRequest(req, config);
    // 记录访问日志
    await logAccess(req, resp, config);
    return resp;
  });
}

// 启动程序
if (import.meta.main) {
  main().catch(console.error);
}
