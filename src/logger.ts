/**
 * 日志模块
 * 提供结构化的日志记录功能
 * 支持日志归档（按大小、按日期、压缩）
 */

import { join, dirname } from "std/path";
import { LogRotator, type RotationConfig } from "./logger-rotation.ts";

/**
 * 日志级别
 */
export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
}

/**
 * 日志级别名称
 */
const LEVEL_NAMES: Record<LogLevel, string> = {
  [LogLevel.DEBUG]: "DEBUG",
  [LogLevel.INFO]: "INFO",
  [LogLevel.WARN]: "WARN",
  [LogLevel.ERROR]: "ERROR",
};

/**
 * ANSI 颜色码
 */
const COLORS = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  dim: "\x1b[2m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  magenta: "\x1b[35m",
  cyan: "\x1b[36m",
  white: "\x1b[37m",
  gray: "\x1b[90m",
};

/**
 * 日志级别对应的颜色
 */
const LEVEL_COLORS: Record<LogLevel, string> = {
  [LogLevel.DEBUG]: COLORS.blue,
  [LogLevel.INFO]: COLORS.green,
  [LogLevel.WARN]: COLORS.yellow,
  [LogLevel.ERROR]: COLORS.red,
};

/**
 * 日志配置
 */
export interface LoggerConfig {
  /** 最小日志级别 */
  minLevel: LogLevel;
  /** 是否启用彩色输出 */
  colorize: boolean;
  /** 输出到控制台 */
  console: boolean;
  /** 输出到文件 */
  file?: string;
  /** 日志格式 */
  format?: "text" | "json";
  /** 日志归档配置 */
  rotation?: RotationConfig;
}

/**
 * 日志记录接口
 */
export interface Logger {
  /** 调试日志 */
  debug(...args: unknown[]): void;
  /** 信息日志 */
  info(...args: unknown[]): void;
  /** 警告日志 */
  warn(...args: unknown[]): void;
  /** 错误日志 */
  error(...args: unknown[]): void;
}

/**
 * 格式化日志消息
 */
function formatMessage(
  level: LogLevel,
  timestamp: Date,
  args: unknown[],
  config: LoggerConfig,
): string {
  const levelName = LEVEL_NAMES[level];
  const timestampStr = timestamp.toISOString();

  if (config.format === "json") {
    const message = args
      .map((arg) =>
        typeof arg === "string"
          ? arg
          : JSON.stringify(arg, (key, value) =>
            typeof value === "bigint" ? `${value}n` : value)
      )
      .join(" ");

    return JSON.stringify({
      level: levelName,
      timestamp: timestampStr,
      message,
    });
  } else {
    // 文本格式
    const message = args
      .map((arg) => typeof arg === "string" ? arg : JSON.stringify(arg))
      .join(" ");

    return `[${timestampStr}] [${levelName}] ${message}`;
  }
}

/**
 * 获取带颜色的日志级别字符串
 */
function getColoredLevel(level: LogLevel, colorize: boolean): string {
  const levelName = LEVEL_NAMES[level];
  if (!colorize) {
    return levelName;
  }
  const color = LEVEL_COLORS[level];
  return `${color}${levelName}${COLORS.reset}`;
}

/**
 * 日志写入器（支持归档）
 */
class LogWriter {
  private rotator?: LogRotator;
  private filepath?: string;

  constructor(config: LoggerConfig) {
    if (config.file) {
      this.filepath = config.file;
      // 如果配置了归档，使用 LogRotator
      if (config.rotation) {
        this.rotator = new LogRotator(config.file, config.rotation);
      }
    }
  }

  async write(message: string): Promise<void> {
    if (!this.filepath) {
      return;
    }

    if (this.rotator) {
      // 使用归档写入器
      await this.rotator.write(message);
    } else {
      // 普通写入（无归档）
      try {
        const logDir = dirname(this.filepath);
        await Deno.mkdir(logDir, { recursive: true });
        await Deno.writeTextFile(this.filepath, message + "\n", {
          append: true,
        });
      } catch (error) {
        console.error(`Failed to write to log file: ${error}`);
      }
    }
  }
}

/**
 * 写入日志到文件（已废弃，使用 LogWriter）
 * @deprecated 使用 LogWriter 代替
 */
async function writeToFile(
  filepath: string,
  message: string,
): Promise<void> {
  try {
    // 每次写入前都确保目录存在
    const logDir = dirname(filepath);
    await Deno.mkdir(logDir, { recursive: true });

    await Deno.writeTextFile(filepath, message + "\n", { append: true });
  } catch (error) {
    console.error(`Failed to write to log file: ${error}`);
  }
}

/**
 * 创建日志记录器
 */
export function createLogger(
  config: LoggerConfig = {
    minLevel: LogLevel.INFO,
    colorize: true,
    console: true,
    format: "text",
  },
): Logger {
  // 创建日志写入器
  const writer = new LogWriter(config);

  return {
    debug(...args: unknown[]) {
      if (config.minLevel > LogLevel.DEBUG) {
        return;
      }

      const timestamp = new Date();
      const message = formatMessage(LogLevel.DEBUG, timestamp, args, config);

      if (config.console) {
        const levelStr = getColoredLevel(LogLevel.DEBUG, config.colorize);
        const colorized = config.colorize
          ? `${COLORS.dim}${message}${COLORS.reset}`
          : message;
        console.log(colorized);
      }

      if (config.file) {
        writer.write(message);
      }
    },

    info(...args: unknown[]) {
      if (config.minLevel > LogLevel.INFO) {
        return;
      }

      const timestamp = new Date();
      const message = formatMessage(LogLevel.INFO, timestamp, args, config);

      if (config.console) {
        const levelStr = getColoredLevel(LogLevel.INFO, config.colorize);
        const colorized = config.colorize ? `${levelStr} ${message}` : message;
        console.log(colorized);
      }

      if (config.file) {
        writer.write(message);
      }
    },

    warn(...args: unknown[]) {
      if (config.minLevel > LogLevel.WARN) {
        return;
      }

      const timestamp = new Date();
      const message = formatMessage(LogLevel.WARN, timestamp, args, config);

      if (config.console) {
        const levelStr = getColoredLevel(LogLevel.WARN, config.colorize);
        const colorized = config.colorize ? `${levelStr} ${message}` : message;
        console.log(colorized);
      }

      if (config.file) {
        writer.write(message);
      }
    },

    error(...args: unknown[]) {
      if (config.minLevel > LogLevel.ERROR) {
        return;
      }

      const timestamp = new Date();
      const message = formatMessage(LogLevel.ERROR, timestamp, args, config);

      if (config.console) {
        const levelStr = getColoredLevel(LogLevel.ERROR, config.colorize);
        const colorized = config.colorize
          ? `${levelStr}${message}${COLORS.reset}`
          : message;
        console.error(colorized);
      }

      if (config.file) {
        writer.write(message);
      }
    },
  };
}

/**
 * 创建默认的日志记录器（用于开发模式）
 */
export function createDefaultLogger(): Logger {
  return createLogger({
    minLevel: LogLevel.INFO,
    colorize: true,
    console: true,
    format: "text",
  });
}

/**
 * 创建生产环境的日志记录器
 * @param config - 日志配置选项
 */
export function createProductionLogger(
  config?: {
    level?: "DEBUG" | "INFO" | "WARN" | "ERROR";
    file?: string;
    colorize?: boolean;
    format?: "text" | "json";
    /** 日志归档配置 */
    rotation?: {
      /** 单个日志文件最大大小（字节），默认 10MB */
      maxSize?: number;
      /** 保留的归档文件数量，默认 5 */
      maxFiles?: number;
      /** 是否压缩归档文件（gzip），默认 false */
      compress?: boolean;
      /** 按日期归档：每天创建新文件，格式：app.log.2025-01-15 */
      daily?: boolean;
    };
  },
): Logger {
  // 解析日志级别
  const logLevel = config?.level || "INFO";
  const minLevel = logLevel === "DEBUG"
    ? LogLevel.DEBUG
    : logLevel === "WARN"
    ? LogLevel.WARN
    : logLevel === "ERROR"
    ? LogLevel.ERROR
    : LogLevel.INFO;

  return createLogger({
    minLevel,
    colorize: config?.colorize ?? false, // 生产模式默认不使用颜色
    console: true,
    format: config?.format || "text",
    file: config?.file,
    rotation: config?.rotation,
  });
}

/**
 * 日志工厂函数类型
 */
export type LoggerFactory = (ctx?: unknown) => Logger;
