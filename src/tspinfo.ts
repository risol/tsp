/**
 * TSP Info 模块
 * 类似 PHP 的 phpinfo()，显示服务器运行时信息
 */

import { getCacheSize } from "./cache.ts";
import { getRegisteredDeps } from "./injection-typed.ts";
import { join } from "std/path";
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
    rotation?: {
      maxSize?: number;
      maxFiles?: number;
      compress?: boolean;
      daily?: boolean;
    };
  };
  /** 运行时信息 */
  runtime: {
    uptime: number;
    uptimeFormatted: string;
    memory: {
      rss: number;
      heapTotal: number;
      heapUsed: number;
      external: number;
      rssFormatted: string;
      heapTotalFormatted: string;
      heapUsedFormatted: string;
      externalFormatted: string;
    };
    pid: number;
    ppid?: number;
    cwd: string;
    execPath?: string;
  };
  /** 系统信息 */
  system: {
    platform: string;
    osVersion: string;
    osRelease?: string;
    hostname: string;
    cpus: {
      physical: number;
      logical: number;
    };
    memTotal: number;
    memTotalFormatted: string;
    memFree?: number;
    memFreeFormatted?: string;
  };
  /** 环境变量（仅显示非敏感的） */
  envVars: Array<{ key: string; value: string }>;
  /** 缓存统计 */
  cache: {
    size: number;
    baseDir: string;
    cacheDir: string;
    compiledFiles?: string[];
  };
  /** 已注册的依赖 */
  dependencies: string[];
  /** 已加载的模块 */
  modules?: Array<{
    name: string;
    version?: string;
  }>;
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
      rotation?: {
        maxSize?: number;
        maxFiles?: number;
        compress?: boolean;
        daily?: boolean;
      };
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
  async getInfo(): Promise<TspInfoData> {
    const cacheSize = getCacheSize();

    // 获取内存使用情况
    const memoryUsage = this.getMemoryUsage();

    // 获取系统信息
    const systemInfo = await this.getSystemInfo();

    // 获取环境变量（仅显示非敏感的）
    const envVars = this.getSafeEnvVars();

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
        rotation: this.config.logger?.rotation,
      },
      runtime: {
        uptime: Math.floor((Date.now() - this.startTime) / 1000),
        uptimeFormatted: this.formatUptime(Math.floor((Date.now() - this.startTime) / 1000)),
        memory: {
          rss: memoryUsage.rss,
          heapTotal: memoryUsage.heapTotal,
          heapUsed: memoryUsage.heapUsed,
          external: memoryUsage.external,
          rssFormatted: this.formatBytes(memoryUsage.rss),
          heapTotalFormatted: this.formatBytes(memoryUsage.heapTotal),
          heapUsedFormatted: this.formatBytes(memoryUsage.heapUsed),
          externalFormatted: this.formatBytes(memoryUsage.external),
        },
        pid: Deno.pid,
        ppid: Deno.ppid,
        cwd: Deno.cwd(),
        execPath: Deno.execPath(),
      },
      system: systemInfo,
      envVars,
      cache: {
        size: cacheSize,
        baseDir: ".cache/tsp",
        cacheDir: this.getCacheDir(),
      },
      dependencies: getRegisteredDeps(),
    };
  }

  /**
   * 渲染 HTML 格式的信息页面（类似 phpinfo）
   */
  async renderHTML(): Promise<string> {
    const info = await this.getInfo();

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
        ${
        info.logging.rotation
          ? `
        <tr>
          <td>归档策略</td>
          <td>
            <span class="value">${
              info.logging.rotation.daily
                ? "按日期归档"
                : `按大小归档 (${this.formatBytes(
                    info.logging.rotation.maxSize || 10485760,
                  )})`
            }</span>
          </td>
        </tr>
        <tr>
          <td>保留文件数</td>
          <td><span class="value">${info.logging.rotation.maxFiles || 5} 个</span></td>
        </tr>
        <tr>
          <td>压缩归档</td>
          <td><span class="value">${
            info.logging.rotation.compress ? "启用 (gzip)" : "未启用"
          }</span></td>
        </tr>
        `
          : ""
      }
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
          <td>父进程 ID</td>
          <td><span class="value">${info.runtime.ppid || "N/A"}</span></td>
        </tr>
        <tr>
          <td>运行时间</td>
          <td><span class="value">${info.runtime.uptimeFormatted}</span></td>
        </tr>
        <tr>
          <td>工作目录</td>
          <td><span class="value">${this.escapeHtml(info.runtime.cwd)}</span></td>
        </tr>
        <tr>
          <td>执行路径</td>
          <td><span class="value">${this.escapeHtml(info.runtime.execPath || "N/A")}</span></td>
        </tr>
        <tr>
          <td>RSS 内存</td>
          <td><span class="value">${info.runtime.memory.rssFormatted}</span></td>
        </tr>
        <tr>
          <td>堆总内存</td>
          <td><span class="value">${info.runtime.memory.heapTotalFormatted}</span></td>
        </tr>
        <tr>
          <td>堆已用内存</td>
          <td><span class="value">${info.runtime.memory.heapUsedFormatted}</span></td>
        </tr>
        <tr>
          <td>外部内存</td>
          <td><span class="value">${info.runtime.memory.externalFormatted}</span></td>
        </tr>
      </table>
    </div>

    <!-- 系统信息 -->
    <div class="section">
      <div class="section-title">🖥️ 系统信息</div>
      <table class="info-table">
        <tr>
          <td>主机名</td>
          <td><span class="value">${info.system.hostname}</span></td>
        </tr>
        <tr>
          <td>操作系统</td>
          <td><span class="value">${info.system.platform}</span></td>
        </tr>
        <tr>
          <td>系统版本</td>
          <td><span class="value">${info.system.osVersion}</span></td>
        </tr>
        <tr>
          <td>CPU 核心数</td>
          <td>
            <span class="value">${info.system.cpus.physical} 物理核心 / ${info.system.cpus.logical} 逻辑核心</span>
            ${info.system.cpus.logical > info.system.cpus.physical
              ? `<span class="badge badge-dev" style="margin-left: 8px;">超线程开启</span>`
              : ""
            }
          </td>
        </tr>
        <tr>
          <td>系统内存</td>
          <td><span class="value">${info.system.memTotalFormatted}</span></td>
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
          <td>缓存基础目录</td>
          <td><span class="value">${info.cache.baseDir}</span></td>
        </tr>
        <tr>
          <td>缓存实际路径</td>
          <td><span class="value">${this.escapeHtml(info.cache.cacheDir)}</span></td>
        </tr>
      </table>
    </div>

    <!-- 环境变量 -->
    <div class="section">
      <div class="section-title">🔧 环境变量 (前 20 个)</div>
      <div class="p-3">
        <div class="code-block" style={{ maxHeight: "300px", overflow: "auto" }}>
          ${info.envVars.map(({ key, value }) => `
            <div style={{ marginBottom: "0.5rem" }}>
              <span class="text-primary">${this.escapeHtml(key)}</span>
              <span class="text-muted"> = </span>
              <span class="value">${this.escapeHtml(value)}</span>
            </div>
          `).join("")}
        </div>
      </div>
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

  /**
   * 获取缓存目录路径
   */
  private getCacheDir(): string {
    try {
      return join(Deno.cwd(), ".cache", "tsp");
    } catch {
      return ".cache/tsp";
    }
  }

  /**
   * 获取内存使用情况
   */
  private getMemoryUsage(): {
    rss: number;
    heapTotal: number;
    heapUsed: number;
    external: number;
  } {
    // Deno 不提供详细的内存信息，返回估算值
    try {
      // 尝试使用性能 API
      if (typeof performance !== "undefined" && (performance as any).memory) {
        const mem = (performance as any).memory;
        return {
          rss: mem.jsHeapSizeLimit || 0,
          heapTotal: mem.jsHeapSizeLimit || 0,
          heapUsed: mem.usedJSHeapSize || 0,
          external: 0,
        };
      }
    } catch {
      // 忽略错误
    }

    return {
      rss: 0,
      heapTotal: 0,
      heapUsed: 0,
      external: 0,
    };
  }

  /**
   * 获取系统信息
   */
  private async getSystemInfo(): Promise<{
    platform: string;
    osVersion: string;
    hostname: string;
    cpus: {
      physical: number;
      logical: number;
    };
    memTotal: number;
    memTotalFormatted: string;
  }> {
    const hostname = Deno.hostname?.() || "unknown";

    // 逻辑核心数（包含超线程）
    const logicalCpus = navigator.hardwareConcurrency || 4;

    // 物理核心数
    const physicalCpus = await this.getPhysicalCpuCount();

    // 获取真实的系统内存
    let memTotal = 8 * 1024 * 1024 * 1024; // 默认 8GB
    try {
      // 尝试使用 node:os 模块获取真实内存
      const os = await import("node:os");
      memTotal = os.totalmem();
    } catch {
      // 如果 node:os 不可用，使用默认值
    }

    return {
      platform: Deno.build.os,
      osVersion: Deno.build.os + " " + Deno.build.arch,
      hostname,
      cpus: {
        physical: physicalCpus,
        logical: logicalCpus,
      },
      memTotal,
      memTotalFormatted: this.formatBytes(memTotal),
    };
  }

  /**
   * 获取物理 CPU 核心数
   */
  private async getPhysicalCpuCount(): Promise<number> {
    try {
      let command: string;

      if (Deno.build.os === "windows") {
        // Windows: 使用 WMIC 获取物理核心数
        command = "wmic cpu get NumberOfCores";
      } else if (Deno.build.os === "linux") {
        // Linux: 读取 /proc/cpuinfo
        command = "grep '^core id' /proc/cpuinfo | sort -u | wc -l";
      } else if (Deno.build.os === "darwin") {
        // macOS: 使用 sysctl
        command = "sysctl -n hw.physicalcpu";
      } else {
        // 未知系统，返回逻辑核心数的一半（估算）
        return Math.ceil((navigator.hardwareConcurrency || 4) / 2);
      }

      const process = new Deno.Command("sh", {
        args: ["-c", command],
        stdout: "piped",
        stderr: "piped",
      });

      const { stdout, code } = await process.output();

      if (code === 0) {
        const output = new TextDecoder().decode(stdout).trim();

        if (Deno.build.os === "windows") {
          // Windows 输出格式：处理多行，找到数字
          const lines = output.split(/\r?\n/);
          for (const line of lines) {
            const num = parseInt(line.trim());
            if (!isNaN(num) && num > 0) {
              return num;
            }
          }
        } else {
          const num = parseInt(output);
          if (!isNaN(num) && num > 0) {
            return num;
          }
        }
      }
    } catch {
      // 忽略错误，使用回退值
    }

    // 回退：如果无法获取物理核心数，返回逻辑核心数的一半（假设超线程开启）
    return Math.ceil((navigator.hardwareConcurrency || 4) / 2);
  }

  /**
   * 获取安全的环境变量（不包含敏感信息）
   */
  private getSafeEnvVars(): Array<{ key: string; value: string }> {
    const safeVars: Array<{ key: string; value: string }> = [];

    // 安全的环境变量前缀
    const safePrefixes = [
      "DENO_",
      "NODE_ENV",
      "PATH",
      "HOME",
      "USER",
      "LANG",
      "LC_",
      "TZ",
    ];

    for (const [key, value] of Object.entries(Deno.env)) {
      // 检查是否为安全变量
      const isSafe = safePrefixes.some((prefix) => key.startsWith(prefix)) ||
        !key.match(/PASSWORD|SECRET|KEY|TOKEN|PRIVATE/i);

      if (isSafe && typeof value === "string" && value.length < 200) {
        safeVars.push({ key, value });
      }
    }

    // 限制数量并排序
    return safeVars
      .slice(0, 20)
      .sort((a, b) => a.key.localeCompare(b.key));
  }

  /**
   * 格式化字节数为可读格式
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
}
