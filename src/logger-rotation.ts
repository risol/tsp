/**
 * Log rotation module
 * Supports automatic log file rotation by size and time
 */

import { dirname, join } from "std/path";

/**
 * Rotation strategy
 */
export interface RotationConfig {
  /** Maximum size of a single log file (bytes), default 10MB */
  maxSize?: number;
  /** Number of archive files to keep, default 5 */
  maxFiles?: number;
  /** Whether to compress archive files (gzip), default false */
  compress?: boolean;
  /** Rotate by date: create new file each day, format: app.log.2025-01-15 */
  daily?: boolean;
}

/**
 * Log state
 */
interface LogState {
  currentSize: number;
  lastCheck: number;
}

/**
 * Log file manager
 */
export class LogRotator {
  private filepath: string;
  private config: Required<RotationConfig>;
  private state: LogState;
  private dailyLogDate: string;

  constructor(filepath: string, config: RotationConfig = {}) {
    this.filepath = filepath;
    this.config = {
      maxSize: config.maxSize || 10 * 1024 * 1024, // Default 10MB
      maxFiles: config.maxFiles || 5,
      compress: config.compress || false,
      daily: config.daily || false,
    };
    this.state = {
      currentSize: 0,
      lastCheck: Date.now(),
    };
    this.dailyLogDate = "";

    // Check current file size on initialization
    this.init();
  }

  /**
   * Initialize log state
   */
  private async init(): Promise<void> {
    try {
      const stat = await Deno.stat(this.filepath);
      this.state.currentSize = stat.size;
    } catch {
      // File doesn't exist, start from 0
      this.state.currentSize = 0;
    }
  }

  /**
   * Write log (handles rotation automatically)
   */
  async write(message: string): Promise<void> {
    // Check if rotation is needed
    if (this.shouldRotate()) {
      await this.rotate();
    }

    // Ensure directory exists
    const logDir = dirname(this.filepath);
    await Deno.mkdir(logDir, { recursive: true });

    // If daily rotation is enabled, use date-based filename
    let targetFile = this.filepath;
    if (this.config.daily) {
      const today = this.getTodayDate();
      if (this.dailyLogDate !== today) {
        // Date changed, archive old file
        if (this.dailyLogDate) {
          await this.archiveDailyLog(this.dailyLogDate);
        }
        this.dailyLogDate = today;
      }
      targetFile = this.getDailyLogPath(today);
    }

    // Write log
    const data = new TextEncoder().encode(message + "\n");
    await Deno.writeFile(targetFile, data, { append: true });
    this.state.currentSize += data.length;
    this.state.lastCheck = Date.now();
  }

  /**
   * Check if rotation is needed
   */
  private shouldRotate(): boolean {
    // If daily rotation is enabled, date check is handled in write
    if (this.config.daily) {
      return false;
    }

    // Check file size
    return this.state.currentSize >= this.config.maxSize;
  }

  /**
   * Perform rotation
   */
  private async rotate(): Promise<void> {
    try {
      // Delete oldest archive file
      const oldestArchive = this.getArchivePath(this.config.maxFiles);
      try {
        await Deno.remove(oldestArchive);
      } catch {
        // File doesn't exist, ignore
      }

      // Rename existing archive files (from old to new)
      for (let i = this.config.maxFiles - 1; i >= 1; i--) {
        const oldArchive = this.getArchivePath(i);
        const newArchive = this.getArchivePath(i + 1);

        try {
          await Deno.rename(oldArchive, newArchive);
        } catch {
          // File doesn't exist, ignore
        }
      }

      // Rename current log file to first archive
      const firstArchive = this.getArchivePath(1);
      try {
        await Deno.rename(this.filepath, firstArchive);

        // If compression is enabled, compress the archive
        if (this.config.compress) {
          await this.compressArchive(firstArchive);
        }
      } catch {
        // Current log file doesn't exist, no need to archive
      }

      // Reset size counter
      this.state.currentSize = 0;
    } catch (error) {
      console.error(`Log rotation failed: ${error}`);
    }
  }

  /**
   * Archive daily log files
   */
  private async archiveDailyLog(date: string): Promise<void> {
    const dailyPath = this.getDailyLogPath(date);

    try {
      // Check if file exists and has content
      const stat = await Deno.stat(dailyPath);
      if (stat.size === 0) {
        return; // Empty file, don't archive
      }

      // Rename to archive file
      const archivePath = this.getArchivePath(date);
      await Deno.rename(dailyPath, archivePath);

      // If compression is enabled, compress the archive
      if (this.config.compress) {
        await this.compressArchive(archivePath);
      }
    } catch (error) {
      // File doesn't exist or other error, ignore
      console.debug(`Failed to archive daily log: ${error}`);
    }

    // Clean up old daily archives
    await this.cleanupOldDailyArchives();
  }

  /**
   * Clean up old daily archive files
   */
  private async cleanupOldDailyArchives(): Promise<void> {
    try {
      const logDir = dirname(this.filepath);
      const baseName = this.getBaseName();
      const entries = Deno.readDir(logDir);

      const archives: string[] = [];
      for await (const entry of entries) {
        if (entry.isFile && entry.name.startsWith(baseName)) {
          const fullPath = join(logDir, entry.name);
          archives.push(fullPath);
        }
      }

      // Sort by modification time (oldest first)
      archives.sort((a, b) => {
        const statA = Deno.statSync(a);
        const statB = Deno.statSync(b);
        return statA.mtime!.getTime() - statB.mtime!.getTime();
      });

      // Delete old archives exceeding maxFiles (keep the newest maxFiles)
      const keepCount = this.config.maxFiles + 1; // +1 because current log file counts too
      if (archives.length > keepCount) {
        const toDelete = archives.slice(0, archives.length - keepCount);
        for (const file of toDelete) {
          try {
            await Deno.remove(file);
          } catch {
            // Ignore delete failures
          }
        }
      }
    } catch (error) {
      console.error(`Failed to clean up old archives: ${error}`);
    }
  }

  /**
   * Compress archive file (gzip)
   */
  private async compressArchive(filepath: string): Promise<void> {
    try {
      const data = await Deno.readFile(filepath);

      // Use Deno's built-in compression
      const compressed = new Response(
        new ReadableStream({
          async start(controller) {
            const compressor = new CompressionStream("gzip");
            const writer = compressor.writable.getWriter();

            try {
              await writer.write(new Uint8Array(data));
              await writer.close();

              const reader = compressor.readable.getReader();

              while (true) {
                const { done, value } = await reader.read();
                if (done) {
                  controller.close();
                  break;
                }
                controller.enqueue(value);
              }
            } catch (error) {
              controller.error(error);
            }
          },
        }),
      ).arrayBuffer();

      // Write compressed file
      const gzipPath = filepath + ".gz";
      await Deno.writeFile(gzipPath, new Uint8Array(await compressed));

      // Delete original file
      await Deno.remove(filepath);
    } catch (error) {
      console.error(`Failed to compress archive: ${error}`);
    }
  }

  /**
   * Get archive file path
   */
  private getArchivePath(indexOrDate: number | string): string {
    const dir = dirname(this.filepath);
    const baseName = this.getBaseName();
    const ext = this.getExtension();
    return join(dir, `${baseName}.${indexOrDate}${ext}`);
  }

  /**
   * Get daily log file path
   */
  private getDailyLogPath(date: string): string {
    const dir = dirname(this.filepath);
    const baseName = this.getBaseName();
    const ext = this.getExtension();
    return join(dir, `${baseName}.${date}${ext}`);
  }

  /**
   * Get base filename (without extension)
   */
  private getBaseName(): string {
    // Use path library to correctly split filename and extension
    const parsed = this.filepath.split(/[/\\]/).pop() || this.filepath;
    const lastDotIndex = parsed.lastIndexOf(".");
    if (lastDotIndex > 0) {
      return parsed.substring(0, lastDotIndex);
    }
    return parsed;
  }

  /**
   * Get file extension (including dot)
   */
  private getExtension(): string {
    const parsed = this.filepath.split(/[/\\]/).pop() || this.filepath;
    const lastDotIndex = parsed.lastIndexOf(".");
    if (lastDotIndex > 0) {
      return parsed.substring(lastDotIndex);
    }
    return "";
  }

  /**
   * Get today's date string
   */
  private getTodayDate(): string {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, "0");
    const day = String(now.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }
}
