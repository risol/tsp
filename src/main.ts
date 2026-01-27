#!/usr/bin/env -S deno run --allow-net --allow-read

/**
 * TSP-FPM: 类 PHP-FPM 模板执行引擎
 * 使用 Deno + TSX 实现的模板服务器
 */

import { resolvePath, securityCheck } from "./router.ts";
import { buildContext } from "./context.ts";
import { getPage, renderJSX, type RedirectResult } from "./cache.ts";

// 配置接口
interface Config {
  root: string;
  port: number;
  dev: boolean;
}

// 默认配置
const DEFAULT_CONFIG: Config = {
  root: "./www",
  port: 9000,
  dev: false,
};

// 解析命令行参数
function parseArgs(): Config {
  const args = Deno.args;
  const config = { ...DEFAULT_CONFIG };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
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
      case "--help":
      case "-h":
        printHelp();
        Deno.exit(0);
        break;
      default:
        console.error(`Unknown option: ${arg}`);
        printHelp();
        Deno.exit(1);
    }
  }

  return config;
}

// 打印帮助信息
function printHelp(): void {
  console.log(`
TSP-FPM: 类 PHP-FPM 模板执行引擎

用法:
  ./tsp-fpm [options]

选项:
  --root, -r <path>   文档根目录 (默认: ./www)
  --port, -p <port>   监听端口 (默认: 9000)
  --dev, -d           开发模式 (显示错误详情)
  --help, -h          显示帮助信息

示例:
  ./tsp-fpm --root ./www --port 9000
  ./tsp-fpm -r ./site -p 8080 --dev
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

    // 解析文件路径
    const fileResult = resolvePath(pathname, config.root);
    if (!fileResult.success) {
      return new Response(fileResult.error, {
        status: 404,
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }

    const filepath = fileResult.filepath!;

    // 安全检查
    const securityResult = await securityCheck(filepath, config.root);
    if (!securityResult.success) {
      return new Response(securityResult.error, {
        status: 403,
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
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

    // 检查是否是 Response 对象（直接返回）
    if (result instanceof Response) {
      return result;
    }

    // 默认：渲染 JSX 为 HTML
    const html = renderJSX(result);
    return new Response(html, {
      status: 200,
      headers: { "Content-Type": "text/html; charset=utf-8" },
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
  const config = parseArgs();

  console.log(`
╔════════════════════════════════════════╗
║         TSP-FPM Template Engine        ║
╚════════════════════════════════════════╝

Document Root: ${config.root}
Port: ${config.port}
Mode: ${config.dev ? "Development" : "Production"}

Starting server...
  `);

  // 启动 HTTP 服务器
  Deno.serve({
    port: config.port,
    onListen: ({ port, hostname }) => {
      console.log(`✓ Server running at http://${hostname}:${port}/`);
      console.log("Press Ctrl+C to stop.\n");
    },
  }, (req) => handleRequest(req, config));
}

// 启动程序
if (import.meta.main) {
  main().catch(console.error);
}
