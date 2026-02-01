/**
 * 日志归档模块
 * 支持按大小、时间自动归档日志文件
 */

import { dirname, join } from "std/path";

/**
 * 归档策略
 */
export interface RotationConfig {
  /** 单个日志文件最大大小（字节），默认 10MB */
  maxSize?: number;
  /** 保留的归档文件数量，默认 5 */
  maxFiles?: number;
  /** 是否压缩归档文件（gzip），默认 false */
  compress?: boolean;
  /** 按日期归档：每天创建新文件，格式：app.log.2025-01-15 */
  daily?: boolean;
}

/**
 * 日志状态
 */
interface LogState {
  currentSize: number;
  lastCheck: number;
}

/**
 * 日志文件管理器
 */
export class LogRotator {
  private filepath: string;
  private config: Required<RotationConfig>;
  private state: LogState;
  private dailyLogDate: string;

  constructor(filepath: string, config: RotationConfig = {}) {
    this.filepath = filepath;
    this.config = {
      maxSize: config.maxSize || 10 * 1024 * 1024, // 默认 10MB
      maxFiles: config.maxFiles || 5,
      compress: config.compress || false,
      daily: config.daily || false,
    };
    this.state = {
      currentSize: 0,
      lastCheck: Date.now(),
    };
    this.dailyLogDate = "";

    // 初始化时检查当前文件大小
    this.init();
  }

  /**
   * 初始化日志状态
   */
  private async init(): Promise<void> {
    try {
      const stat = await Deno.stat(this.filepath);
      this.state.currentSize = stat.size;
    } catch {
      // 文件不存在，从 0 开始
      this.state.currentSize = 0;
    }
  }

  /**
   * 写入日志（自动处理归档）
   */
  async write(message: string): Promise<void> {
    // 检查是否需要归档
    if (this.shouldRotate()) {
      await this.rotate();
    }

    // 确保目录存在
    const logDir = dirname(this.filepath);
    await Deno.mkdir(logDir, { recursive: true });

    // 如果启用了按日期归档，使用日期文件名
    let targetFile = this.filepath;
    if (this.config.daily) {
      const today = this.getTodayDate();
      if (this.dailyLogDate !== today) {
        // 日期改变，归档旧文件
        if (this.dailyLogDate) {
          await this.archiveDailyLog(this.dailyLogDate);
        }
        this.dailyLogDate = today;
      }
      targetFile = this.getDailyLogPath(today);
    }

    // 写入日志
    const data = new TextEncoder().encode(message + "\n");
    await Deno.writeFile(targetFile, data, { append: true });
    this.state.currentSize += data.length;
    this.state.lastCheck = Date.now();
  }

  /**
   * 检查是否需要归档
   */
  private shouldRotate(): boolean {
    // 如果启用按日期归档，日期检查在 write 中处理
    if (this.config.daily) {
      return false;
    }

    // 检查文件大小
    return this.state.currentSize >= this.config.maxSize;
  }

  /**
   * 执行归档
   */
  private async rotate(): Promise<void> {
    try {
      // 删除最旧的归档文件
      const oldestArchive = this.getArchivePath(this.config.maxFiles);
      try {
        await Deno.remove(oldestArchive);
      } catch {
        // 文件不存在，忽略
      }

      // 重命名现有归档文件（从旧到新）
      for (let i = this.config.maxFiles - 1; i >= 1; i--) {
        const oldArchive = this.getArchivePath(i);
        const newArchive = this.getArchivePath(i + 1);

        try {
          await Deno.rename(oldArchive, newArchive);
        } catch {
          // 文件不存在，忽略
        }
      }

      // 重命名当前日志文件为第一个归档
      const firstArchive = this.getArchivePath(1);
      try {
        await Deno.rename(this.filepath, firstArchive);

        // 如果启用压缩，压缩归档文件
        if (this.config.compress) {
          await this.compressArchive(firstArchive);
        }
      } catch {
        // 当前日志文件不存在，无需归档
      }

      // 重置大小计数
      this.state.currentSize = 0;
    } catch (error) {
      console.error(`日志归档失败: ${error}`);
    }
  }

  /**
   * 归档按日期的日志文件
   */
  private async archiveDailyLog(date: string): Promise<void> {
    const dailyPath = this.getDailyLogPath(date);

    try {
      // 检查文件是否存在且有内容
      const stat = await Deno.stat(dailyPath);
      if (stat.size === 0) {
        return; // 空文件不归档
      }

      // 重命名为归档文件
      const archivePath = this.getArchivePath(date);
      await Deno.rename(dailyPath, archivePath);

      // 如果启用压缩，压缩归档文件
      if (this.config.compress) {
        await this.compressArchive(archivePath);
      }
    } catch (error) {
      // 文件不存在或其他错误，忽略
      console.debug(`归档日期日志失败: ${error}`);
    }

    // 清理旧的日期归档
    await this.cleanupOldDailyArchives();
  }

  /**
   * 清理旧的日期归档文件
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

      // 按修改时间排序（从旧到新）
      archives.sort((a, b) => {
        const statA = Deno.statSync(a);
        const statB = Deno.statSync(b);
        return statA.mtime!.getTime() - statB.mtime!.getTime();
      });

      // 删除超过 maxFiles 的旧归档（保留最新的 maxFiles 个）
      const keepCount = this.config.maxFiles + 1; // +1 因为当前日志文件也算
      if (archives.length > keepCount) {
        const toDelete = archives.slice(0, archives.length - keepCount);
        for (const file of toDelete) {
          try {
            await Deno.remove(file);
          } catch {
            // 忽略删除失败
          }
        }
      }
    } catch (error) {
      console.error(`清理旧归档失败: ${error}`);
    }
  }

  /**
   * 压缩归档文件（gzip）
   */
  private async compressArchive(filepath: string): Promise<void> {
    try {
      const data = await Deno.readFile(filepath);

      // 使用 Deno 内置的压缩功能
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

      // 写入压缩文件
      const gzipPath = filepath + ".gz";
      await Deno.writeFile(gzipPath, new Uint8Array(await compressed));

      // 删除原始文件
      await Deno.remove(filepath);
    } catch (error) {
      console.error(`压缩归档文件失败: ${error}`);
    }
  }

  /**
   * 获取归档文件路径
   */
  private getArchivePath(indexOrDate: number | string): string {
    const dir = dirname(this.filepath);
    const baseName = this.getBaseName();
    const ext = this.getExtension();
    return join(dir, `${baseName}.${indexOrDate}${ext}`);
  }

  /**
   * 获取按日期归档的文件路径
   */
  private getDailyLogPath(date: string): string {
    const dir = dirname(this.filepath);
    const baseName = this.getBaseName();
    const ext = this.getExtension();
    return join(dir, `${baseName}.${date}${ext}`);
  }

  /**
   * 获取基础文件名（不含扩展名）
   */
  private getBaseName(): string {
    // 使用路径处理库正确分割文件名和扩展名
    const parsed = this.filepath.split(/[/\\]/).pop() || this.filepath;
    const lastDotIndex = parsed.lastIndexOf(".");
    if (lastDotIndex > 0) {
      return parsed.substring(0, lastDotIndex);
    }
    return parsed;
  }

  /**
   * 获取文件扩展名（包含点）
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
   * 获取今天的日期字符串
   */
  private getTodayDate(): string {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, "0");
    const day = String(now.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }
}
