/**
 * Logger module
 * Provides structured logging functionality
 * Supports log rotation (by size, by date, compression)
 */

import { join, dirname } from "std/path";
import { LogRotator, type RotationConfig } from "./logger-rotation.ts";

/**
 * Log level
 */
export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
}

/**
 * Log level names
 */
const LEVEL_NAMES: Record<LogLevel, string> = {
  [LogLevel.DEBUG]: "DEBUG",
  [LogLevel.INFO]: "INFO",
  [LogLevel.WARN]: "WARN",
  [LogLevel.ERROR]: "ERROR",
};

/**
 * ANSI color codes
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
 * Color for each log level
 */
const LEVEL_COLORS: Record<LogLevel, string> = {
  [LogLevel.DEBUG]: COLORS.blue,
  [LogLevel.INFO]: COLORS.green,
  [LogLevel.WARN]: COLORS.yellow,
  [LogLevel.ERROR]: COLORS.red,
};

/**
 * Logger configuration
 */
export interface LoggerConfig {
  /** Minimum log level */
  minLevel: LogLevel;
  /** Enable colored output */
  colorize: boolean;
  /** Output to console */
  console: boolean;
  /** Output to file */
  file?: string;
  /** Log format */
  format?: "text" | "json";
  /** Log rotation configuration */
  rotation?: RotationConfig;
}

/**
 * Logger interface
 */
export interface Logger {
  /** Debug log */
  debug(...args: unknown[]): void;
  /** Info log */
  info(...args: unknown[]): void;
  /** Warning log */
  warn(...args: unknown[]): void;
  /** Error log */
  error(...args: unknown[]): void;
}

/**
 * Format log message
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
    // Text format
    const message = args
      .map((arg) => typeof arg === "string" ? arg : JSON.stringify(arg))
      .join(" ");

    return `[${timestampStr}] [${levelName}] ${message}`;
  }
}

/**
 * Get colored log level string
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
 * Log writer (with rotation support)
 */
class LogWriter {
  private rotator?: LogRotator;
  private filepath?: string;

  constructor(config: LoggerConfig) {
    if (config.file) {
      this.filepath = config.file;
      // If rotation is configured, use LogRotator
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
      // Use rotation writer
      await this.rotator.write(message);
    } else {
      // Normal write (no rotation)
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
 * Write log to file (deprecated, use LogWriter)
 * @deprecated Use LogWriter instead
 */
async function writeToFile(
  filepath: string,
  message: string,
): Promise<void> {
  try {
    // Ensure directory exists before each write
    const logDir = dirname(filepath);
    await Deno.mkdir(logDir, { recursive: true });

    await Deno.writeTextFile(filepath, message + "\n", { append: true });
  } catch (error) {
    console.error(`Failed to write to log file: ${error}`);
  }
}

/**
 * Create logger instance
 */
export function createLogger(
  config: LoggerConfig = {
    minLevel: LogLevel.INFO,
    colorize: true,
    console: true,
    format: "text",
  },
): Logger {
  // Create log writer
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
 * Create default logger (for development mode)
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
 * Create production logger
 * @param config - Logger configuration options
 */
export function createProductionLogger(
  config?: {
    level?: "DEBUG" | "INFO" | "WARN" | "ERROR";
    file?: string;
    colorize?: boolean;
    format?: "text" | "json";
    /** Log rotation configuration */
    rotation?: {
      /** Maximum size per log file (bytes), default 10MB */
      maxSize?: number;
      /** Number of archived files to keep, default 5 */
      maxFiles?: number;
      /** Whether to compress archived files (gzip), default false */
      compress?: boolean;
      /** Daily rotation: create new file each day, format: app.log.2025-01-15 */
      daily?: boolean;
    };
  },
): Logger {
  // Parse log level
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
    colorize: config?.colorize ?? false, // Production mode defaults to no colors
    console: true,
    format: config?.format || "text",
    file: config?.file,
    rotation: config?.rotation,
  });
}

/**
 * Logger factory function type
 */
export type LoggerFactory = (ctx?: unknown) => Logger;
