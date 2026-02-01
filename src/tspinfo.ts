/**
 * TSP Info 模块
 * 类似 PHP 的 phpinfo()，显示服务器运行时信息
 */

import { getCacheSize } from "./cache.ts";
import { getRegisteredDeps } from "./injection-typed.ts";
import type { Logger } from "./logger.ts";

/**
 * TSP 信息接口
 */
export interface TspInfoData {
  /** 服务器信息 */
  server: {
    version: string;
    denoVersion: string;
    architecture: string;
    target: string;
  };
  /** 配置信息 */
  config: {
    root: string;
    port: number;
    mode: string;
    accessLog?: string;
  };
  /** 日志配置 */
  logging: {
    level: string;
    file?: string;
    format: string;
  };
  /** 运行时信息 */
  runtime: {
    uptime: number;
    memory: {
      rss: number;
      heapTotal: number;
      heapUsed: number;
      external: number;
    };
    pid: number;
    cwd: string;
  };
  /** 缓存统计 */
  cache: {
    size: number;
    baseDir: string;
  };
  /** 已注册的依赖 */
  dependencies: string[];
}

/**
 * TSP Info 类
 */
export class TspInfo {
  private startTime: number;
  private config: {
    root: string;
    port: number;
    dev: boolean;
    accessLogPath?: string;
    logger?: {
      level?: string;
      file?: string;
      format?: string;
    };
  };
  private logger?: Logger;

  constructor(
    config: {
      root: string;
      port: number;
      dev: boolean;
      accessLogPath?: string;
      logger?: {
        level?: string;
        file?: string;
        format?: string;
      };
    },
    logger?: Logger,
  ) {
    this.startTime = Date.now();
    this.config = config;
    this.logger = logger;
  }

  /**
   * 获取完整的 TSP 信息
   */
  getInfo(): TspInfoData {
    const cacheSize = getCacheSize();

    return {
      server: {
        version: Deno.version.deno,
        denoVersion: Deno.version.deno,
        architecture: Deno.build.arch,
        target: Deno.build.target,
      },
      config: {
        root: this.config.root,
        port: this.config.port,
        mode: this.config.dev ? "Development" : "Production",
        accessLog: this.config.accessLogPath,
      },
      logging: {
        level: this.config.logger?.level || "INFO",
        file: this.config.logger?.file,
        format: this.config.logger?.format || "text",
      },
      runtime: {
        uptime: Math.floor((Date.now() - this.startTime) / 1000),
        memory: {
          rss: 0, // Deno 不提供 process.memoryUsage()
          heapTotal: 0,
          heapUsed: 0,
          external: 0,
        },
        pid: Deno.pid,
        cwd: Deno.cwd(),
      },
      cache: {
        size: cacheSize,
        baseDir: ".cache/tsp",
      },
      dependencies: getRegisteredDeps(),
    };
  }

  /**
   * 渲染 HTML 格式的信息页面（类似 phpinfo）
   */
  renderHTML(): string {
    const info = this.getInfo();

    return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>TSP Info</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      font-size: 14px;
      line-height: 1.6;
      color: #333;
      background: #f5f5f5;
      padding: 20px;
    }
    .container {
      max-width: 1000px;
      margin: 0 auto;
      background: white;
      border-radius: 8px;
      box-shadow: 0 2px 8px rgba(0,0,0,0.1);
      overflow: hidden;
    }
    .header {
      background: #667eea;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      color: white;
      padding: 30px;
      text-align: center;
    }
    .header h1 {
      font-size: 36px;
      margin-bottom: 10px;
    }
    .header p {
      opacity: 0.9;
      font-size: 16px;
    }
    .section {
      border-bottom: 1px solid #e0e0e0;
    }
    .section:last-child {
      border-bottom: none;
    }
    .section-title {
      background: #f8f9fa;
      padding: 15px 20px;
      font-size: 18px;
      font-weight: 600;
      color: #333;
      border-bottom: 1px solid #e0e0e0;
    }
    .info-table {
      width: 100%;
      border-collapse: collapse;
    }
    .info-table tr {
      border-bottom: 1px solid #f0f0f0;
    }
    .info-table tr:last-child {
      border-bottom: none;
    }
    .info-table td {
      padding: 12px 20px;
    }
    .info-table td:first-child {
      font-weight: 600;
      color: #555;
      width: 200px;
      background: #fafafa;
    }
    .info-table td:last-child {
      color: #333;
    }
    .value {
      font-family: "Monaco", "Menlo", "Ubuntu Mono", monospace;
      font-size: 13px;
      background: #f5f5f5;
      padding: 2px 6px;
      border-radius: 3px;
    }
    .badge {
      display: inline-block;
      padding: 4px 8px;
      border-radius: 4px;
      font-size: 12px;
      font-weight: 600;
    }
    .badge-dev {
      background: #fff3cd;
      color: #856404;
    }
    .badge-prod {
      background: #d4edda;
      color: #155724;
    }
    .dependency-list {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      padding: 20px;
    }
    .dependency-item {
      background: #e7f3ff;
      color: #004085;
      padding: 6px 12px;
      border-radius: 4px;
      font-family: "Monaco", "Menlo", "Ubuntu Mono", monospace;
      font-size: 13px;
    }
    .footer {
      text-align: center;
      padding: 20px;
      color: #666;
      font-size: 13px;
      background: #f8f9fa;
      border-top: 1px solid #e0e0e0;
    }
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>🚀 TSP Info</h1>
      <p>TypeScript Server Page - 运行时信息</p>
    </div>

    <!-- 服务器信息 -->
    <div class="section">
      <div class="section-title">📡 服务器信息</div>
      <table class="info-table">
        <tr>
          <td>TSP 版本</td>
          <td><span class="value">${info.server.version}</span></td>
        </tr>
        <tr>
          <td>Deno 版本</td>
          <td><span class="value">${info.server.denoVersion}</span></td>
        </tr>
        <tr>
          <td>系统架构</td>
          <td><span class="value">${info.server.architecture}</span></td>
        </tr>
        <tr>
          <td>编译目标</td>
          <td><span class="value">${info.server.target}</span></td>
        </tr>
      </table>
    </div>

    <!-- 配置信息 -->
    <div class="section">
      <div class="section-title">⚙️ 配置信息</div>
      <table class="info-table">
        <tr>
          <td>文档根目录</td>
          <td><span class="value">${this.escapeHtml(info.config.root)}</span></td>
        </tr>
        <tr>
          <td>监听端口</td>
          <td><span class="value">${info.config.port}</span></td>
        </tr>
        <tr>
          <td>运行模式</td>
          <td>
            <span class="badge ${info.config.mode === "Development" ? "badge-dev" : "badge-prod"}">
              ${info.config.mode}
            </span>
          </td>
        </tr>
        <tr>
          <td>访问日志</td>
          <td><span class="value">${info.config.accessLog || "未配置（控制台输出）"}</span></td>
        </tr>
      </table>
    </div>

    <!-- 日志配置 -->
    <div class="section">
      <div class="section-title">📝 日志配置</div>
      <table class="info-table">
        <tr>
          <td>日志级别</td>
          <td><span class="value">${info.logging.level}</span></td>
        </tr>
        <tr>
          <td>日志格式</td>
          <td><span class="value">${info.logging.format}</span></td>
        </tr>
        <tr>
          <td>日志文件</td>
          <td><span class="value">${info.logging.file || "未配置（控制台输出）"}</span></td>
        </tr>
      </table>
    </div>

    <!-- 运行时信息 -->
    <div class="section">
      <div class="section-title">💻 运行时信息</div>
      <table class="info-table">
        <tr>
          <td>进程 ID</td>
          <td><span class="value">${info.runtime.pid}</span></td>
        </tr>
        <tr>
          <td>工作目录</td>
          <td><span class="value">${this.escapeHtml(info.runtime.cwd)}</span></td>
        </tr>
        <tr>
          <td>运行时间</td>
          <td><span class="value">${this.formatUptime(info.runtime.uptime)}</span></td>
        </tr>
      </table>
    </div>

    <!-- 缓存统计 -->
    <div class="section">
      <div class="section-title">🗄️ 缓存统计</div>
      <table class="info-table">
        <tr>
          <td>缓存文件数</td>
          <td><span class="value">${info.cache.size}</span></td>
        </tr>
        <tr>
          <td>缓存目录</td>
          <td><span class="value">${info.cache.baseDir}</span></td>
        </tr>
      </table>
    </div>

    <!-- 已注册的依赖 -->
    <div class="section">
      <div class="section-title">📦 已注册的依赖 (${info.dependencies.length})</div>
      <div class="dependency-list">
        ${info.dependencies.map(dep => `<div class="dependency-item">${this.escapeHtml(dep)}</div>`).join("")}
      </div>
    </div>

    <!-- 页脚 -->
    <div class="footer">
      <p>TSP Server - TypeScript Server Page</p>
      <p style="margin-top: 5px; opacity: 0.8;">基于 Deno + TSX + Preact 构建</p>
    </div>
  </div>
</body>
</html>`;
  }

  /**
   * 转义 HTML 特殊字符
   */
  private escapeHtml(text: string): string {
    const map: Record<string, string> = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#039;",
    };
    return text.replace(/[&<>"']/g, (m) => map[m]);
  }

  /**
   * 格式化字节数
   */
  private formatBytes(bytes: number): string {
    const units = ["B", "KB", "MB", "GB"];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }

    return `${size.toFixed(2)} ${units[unitIndex]}`;
  }

  /**
   * 格式化运行时间
   */
  private formatUptime(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;

    if (hours > 0) {
      return `${hours} 小时 ${minutes} 分钟 ${secs} 秒`;
    } else if (minutes > 0) {
      return `${minutes} 分钟 ${secs} 秒`;
    } else {
      return `${secs} 秒`;
    }
  }
}
