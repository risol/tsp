/**
 * TSP Info module
 * Similar to PHP's phpinfo(), displays server runtime information
 */

import { getRegisteredDeps } from "./injection-typed.ts";
import { join } from "std/path";
import type { Logger } from "./logger.ts";

/**
 * TSP info interface
 */
export interface TspInfoData {
  /** Server information */
  server: {
    tspVersion: string;
    architecture: string;
    target: string;
  };
  /** Configuration information */
  config: {
    root: string;
    port: number;
    mode: string;
    accessLog?: {
      file?: string;
      rotation?: {
        maxSize?: number;
        maxFiles?: number;
        compress?: boolean;
        daily?: boolean;
      };
    };
  };
  /** Logging configuration */
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
  /** Runtime information */
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
  /** System information */
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
  /** Environment variables (only show non-sensitive) */
  envVars: Array<{ key: string; value: string }>;
  /** Registered dependencies */
  dependencies: string[];
  /** Loaded modules */
  modules?: Array<{
    name: string;
    version?: string;
  }>;
}

/**
 * TSP Info class
 */
export class TspInfo {
  private startTime: number;
  private tspVersion: string;
  private config: {
    root: string;
    port: number;
    dev: boolean;
    accessLog?: {
      file?: string;
      rotation?: {
        maxSize?: number;
        maxFiles?: number;
        compress?: boolean;
        daily?: boolean;
      };
    };
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
    tspVersion: string,
    config: {
      root: string;
      port: number;
      dev: boolean;
      accessLog?: {
        file?: string;
        rotation?: {
          maxSize?: number;
          maxFiles?: number;
          compress?: boolean;
          daily?: boolean;
        };
      };
      logger?: {
        level?: string;
        file?: string;
        format?: string;
      };
    },
    logger?: Logger,
  ) {
    this.startTime = Date.now();
    this.tspVersion = tspVersion;
    this.config = config;
    this.logger = logger;
  }

  /**
   * Get full TSP information
   */
  async getData(): Promise<TspInfoData> {
    return this.getInfo();
  }

  /**
   * Get full TSP information (internal method)
   */
  private async getInfo(): Promise<TspInfoData> {
    // Get memory usage
    const memoryUsage = this.getMemoryUsage();

    // Get system info
    const systemInfo = await this.getSystemInfo();

    // Get environment variables (only show non-sensitive)
    const envVars = this.getSafeEnvVars();

    return {
      server: {
        tspVersion: this.tspVersion,
        architecture: Deno.build.arch,
        target: Deno.build.target,
      },
      config: {
        root: this.config.root,
        port: this.config.port,
        mode: this.config.dev ? "Development" : "Production",
        accessLog: this.config.accessLog,
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
      dependencies: getRegisteredDeps(),
    };
  }

  /**
   * Render HTML format info page (similar to phpinfo)
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
      <p>TypeScript Server Page - Runtime Information</p>
    </div>

    <!-- Server Information -->
    <div class="section">
      <div class="section-title">Server Information</div>
      <table class="info-table">
        <tr>
          <td>TSP Version</td>
          <td><span class="value">${info.server.tspVersion}</span></td>
        </tr>
        <tr>
          <td>System Architecture</td>
          <td><span class="value">${info.server.architecture}</span></td>
        </tr>
        <tr>
          <td>Build Target</td>
          <td><span class="value">${info.server.target}</span></td>
        </tr>
      </table>
    </div>

    <!-- Configuration Information -->
    <div class="section">
      <div class="section-title">Configuration</div>
      <table class="info-table">
        <tr>
          <td>Document Root</td>
          <td><span class="value">${this.escapeHtml(info.config.root)}</span></td>
        </tr>
        <tr>
          <td>Port</td>
          <td><span class="value">${info.config.port}</span></td>
        </tr>
        <tr>
          <td>Mode</td>
          <td>
            <span class="badge ${info.config.mode === "Development" ? "badge-dev" : "badge-prod"}">
              ${info.config.mode}
            </span>
          </td>
        </tr>
        <tr>
          <td>Access Log</td>
          <td><span class="value">${info.config.accessLog?.file ? info.config.accessLog.file + (info.config.accessLog.rotation ? ` (rotation: ${info.config.accessLog.rotation.maxSize ? Math.round(info.config.accessLog.rotation.maxSize/1024/1024) + 'MB' : '10MB'}, maxFiles: ${info.config.accessLog.rotation.maxFiles || 5})` : '') : "Console output"}</span></td>
        </tr>
      </table>
    </div>

    <!-- Logging Configuration -->
    <div class="section">
      <div class="section-title">Logging Configuration</div>
      <table class="info-table">
        <tr>
          <td>Log Level</td>
          <td><span class="value">${info.logging.level}</span></td>
        </tr>
        <tr>
          <td>Log Format</td>
          <td><span class="value">${info.logging.format}</span></td>
        </tr>
        <tr>
          <td>Log File</td>
          <td><span class="value">${info.logging.file || "Not configured (console output)"}</span></td>
        </tr>
        ${
        info.logging.rotation
          ? `
        <tr>
          <td>Archive Strategy</td>
          <td>
            <span class="value">${
              info.logging.rotation.daily
                ? "Daily rotation"
                : `Size-based (${this.formatBytes(
                    info.logging.rotation.maxSize || 10485760,
                  )})`
            }</span>
          </td>
        </tr>
        <tr>
          <td>Max Files</td>
          <td><span class="value">${info.logging.rotation.maxFiles || 5} files</span></td>
        </tr>
        <tr>
          <td>Compress Archives</td>
          <td><span class="value">${
            info.logging.rotation.compress ? "Enabled (gzip)" : "Disabled"
          }</span></td>
        </tr>
        `
          : ""
      }
      </table>
    </div>

    <!-- Runtime Information -->
    <div class="section">
      <div class="section-title">Runtime Information</div>
      <table class="info-table">
        <tr>
          <td>Process ID</td>
          <td><span class="value">${info.runtime.pid}</span></td>
        </tr>
        <tr>
          <td>Parent Process ID</td>
          <td><span class="value">${info.runtime.ppid || "N/A"}</span></td>
        </tr>
        <tr>
          <td>Uptime</td>
          <td><span class="value">${info.runtime.uptimeFormatted}</span></td>
        </tr>
        <tr>
          <td>Working Directory</td>
          <td><span class="value">${this.escapeHtml(info.runtime.cwd)}</span></td>
        </tr>
        <tr>
          <td>Executable Path</td>
          <td><span class="value">${this.escapeHtml(info.runtime.execPath || "N/A")}</span></td>
        </tr>
        <tr>
          <td>RSS Memory</td>
          <td><span class="value">${info.runtime.memory.rssFormatted}</span></td>
        </tr>
        <tr>
          <td>Heap Total</td>
          <td><span class="value">${info.runtime.memory.heapTotalFormatted}</span></td>
        </tr>
        <tr>
          <td>Heap Used</td>
          <td><span class="value">${info.runtime.memory.heapUsedFormatted}</span></td>
        </tr>
        <tr>
          <td>External Memory</td>
          <td><span class="value">${info.runtime.memory.externalFormatted}</span></td>
        </tr>
      </table>
    </div>

    <!-- System Information -->
    <div class="section">
      <div class="section-title">System Information</div>
      <table class="info-table">
        <tr>
          <td>Hostname</td>
          <td><span class="value">${info.system.hostname}</span></td>
        </tr>
        <tr>
          <td>Operating System</td>
          <td><span class="value">${info.system.platform}</span></td>
        </tr>
        <tr>
          <td>OS Version</td>
          <td><span class="value">${info.system.osVersion}</span></td>
        </tr>
        <tr>
          <td>CPU Cores</td>
          <td>
            <span class="value">${info.system.cpus.physical} Physical / ${info.system.cpus.logical} Logical</span>
            ${info.system.cpus.logical > info.system.cpus.physical
              ? `<span class="badge badge-dev" style="margin-left: 8px;">Hyper-threading enabled</span>`
              : ""
            }
          </td>
        </tr>
        <tr>
          <td>System Memory</td>
          <td><span class="value">${info.system.memTotalFormatted}</span></td>
        </tr>
      </table>
    </div>

    <!-- Environment Variables -->
    <div class="section">
      <div class="section-title">Environment Variables (Top 20)</div>
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

    <!-- Registered Dependencies -->
    <div class="section">
      <div class="section-title">Registered Dependencies (${info.dependencies.length})</div>
      <div class="dependency-list">
        ${info.dependencies.map(dep => `<div class="dependency-item">${this.escapeHtml(dep)}</div>`).join("")}
      </div>
    </div>

    <!-- Footer -->
    <div class="footer">
      <p>TSP Server - TypeScript Server Page</p>
      <p style="margin-top: 5px; opacity: 0.8;">Built with Deno + TSX + React</p>
    </div>
  </div>
</body>
</html>`;
  }

  /**
   * Escape HTML special characters
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
   * Format uptime
   */
  private formatUptime(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;

    if (hours > 0) {
      return `${hours} hours ${minutes} minutes ${secs} seconds`;
    } else if (minutes > 0) {
      return `${minutes} minutes ${secs} seconds`;
    } else {
      return `${secs} seconds`;
    }
  }

  /**
   * Get memory usage
   */
  private getMemoryUsage(): {
    rss: number;
    heapTotal: number;
    heapUsed: number;
    external: number;
  } {
    // Deno doesn't provide detailed memory info, return estimated values
    try {
      // Try to use performance API
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
      // Ignore errors
    }

    return {
      rss: 0,
      heapTotal: 0,
      heapUsed: 0,
      external: 0,
    };
  }

  /**
   * Get system information
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

    // Logical CPU cores (including hyper-threading)
    const logicalCpus = navigator.hardwareConcurrency || 4;

    // Physical CPU cores
    const physicalCpus = await this.getPhysicalCpuCount();

    // Get actual system memory
    let memTotal = 8 * 1024 * 1024 * 1024; // Default 8GB
    try {
      // Try to use node:os module to get actual memory
      const os = await import("node:os");
      memTotal = os.totalmem();
    } catch {
      // If node:os is not available, use default value
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
   * Get physical CPU core count
   */
  private async getPhysicalCpuCount(): Promise<number> {
    try {
      let command: string;

      if (Deno.build.os === "windows") {
        // Windows: Use WMIC to get physical core count
        command = "wmic cpu get NumberOfCores";
      } else if (Deno.build.os === "linux") {
        // Linux: Read /proc/cpuinfo
        command = "grep '^core id' /proc/cpuinfo | sort -u | wc -l";
      } else if (Deno.build.os === "darwin") {
        // macOS: Use sysctl
        command = "sysctl -n hw.physicalcpu";
      } else {
        // Unknown system, return half of logical cores (estimated)
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
          // Windows output format: handle multiple lines, find numbers
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
      // Ignore errors, use fallback value
    }

    // Fallback: If unable to get physical core count, return half of logical cores (assuming hyper-threading is enabled)
    return Math.ceil((navigator.hardwareConcurrency || 4) / 2);
  }

  /**
   * Get safe environment variables (excluding sensitive information)
   */
  private getSafeEnvVars(): Array<{ key: string; value: string }> {
    const safeVars: Array<{ key: string; value: string }> = [];

    // Safe environment variable prefixes
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
      // Check if it's a safe variable
      const isSafe = safePrefixes.some((prefix) => key.startsWith(prefix)) ||
        !key.match(/PASSWORD|SECRET|KEY|TOKEN|PRIVATE/i);

      if (isSafe && typeof value === "string" && value.length < 200) {
        safeVars.push({ key, value });
      }
    }

    // Limit count and sort
    return safeVars
      .slice(0, 20)
      .sort((a, b) => a.key.localeCompare(b.key));
  }

  /**
   * Format bytes to human-readable format
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
