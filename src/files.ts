/**
 * 文件上传处理模块
 * 提供 multipart/form-data 解析和文件保存功能
 */

import { join } from "std/path";
import { nanoid } from "nanoid";

/**
 * 上传的文件信息
 */
export interface UploadedFile {
  /** 原始文件名 */
  name: string;
  /** MIME 类型 */
  type: string;
  /** 文件大小（字节） */
  size: number;
  /** 文件内容（Uint8Array） */
  data: Uint8Array;
  /** 保存文件到指定路径 */
  save(path: string): Promise<void>;
  /** 转换为文本（适用于文本文件） */
  text(): Promise<string>;
}

/**
 * 创建上传文件对象
 */
export function createUploadedFile(
  name: string,
  type: string,
  data: Uint8Array,
): UploadedFile {
  const file = {
    name,
    type,
    size: data.byteLength,
    data,

    /**
     * 保存文件到指定路径
     * @param path - 目标路径（可以是相对路径或绝对路径）
     */
    async save(path: string): Promise<void> {
      await Deno.writeFile(path, this.data);
    },

    /**
     * 将文件内容转换为文本
     */
    async text(): Promise<string> {
      const decoder = new TextDecoder();
      return decoder.decode(this.data);
    },
  };

  return file;
}

/**
 * 文件上传配置
 */
export interface FileUploadOptions {
  /** 最大文件大小（字节），默认 10MB */
  maxSize?: number;
  /** 允许的文件类型（MIME 类型），默认允许所有类型 */
  allowedTypes?: string[];
  /** 上传目录（用于默认保存行为），默认 "./uploads" */
  uploadDir?: string;
}

/**
 * 解析 multipart/form-data 请求体
 * @param requestBody - 请求体（Uint8Array）
 * @param contentType - Content-Type 头（包含 boundary）
 * @param options - 上传配置选项
 * @returns 解析后的表单数据和文件
 */
export async function parseMultipartFormData(
  requestBody: Uint8Array,
  contentType: string,
  options: FileUploadOptions = {},
): Promise<{
  fields: Record<string, string>;
  files: Record<string, UploadedFile | UploadedFile[]>;
}> {
  // 提取 boundary
  const boundaryMatch = contentType.match(/boundary=([^;]+)/i);
  if (!boundaryMatch) {
    throw new Error("Invalid Content-Type: missing boundary");
  }

  const boundary = boundaryMatch[1];
  const fields: Record<string, string> = {};
  const files: Record<string, UploadedFile | UploadedFile[]> = {};

  // 解析 multipart 数据
  const parts = parseMultipartBody(requestBody, boundary);

  for (const part of parts) {
    // 检查文件大小限制
    const maxSize = options.maxSize || 10 * 1024 * 1024; // 默认 10MB
    if (part.data.byteLength > maxSize) {
      throw new Error(
        `File "${part.name || "unknown"}" exceeds maximum size of ${maxSize} bytes`,
      );
    }

    // 如果有 filename，则是文件上传
    if (part.filename) {
      // 检查文件类型限制
      if (options.allowedTypes && part.type) {
        if (!options.allowedTypes.includes(part.type)) {
          throw new Error(
            `File type "${part.type}" is not allowed. Allowed types: ${
              options.allowedTypes.join(", ")
            }`,
          );
        }
      }

      const file = createUploadedFile(part.filename, part.type, part.data);

      // 如果是数组字段（多个文件同名字段），处理为数组
      if (part.name && files[part.name]) {
        // 已存在同名文件，转换为数组
        if (!Array.isArray(files[part.name])) {
          files[part.name] = [files[part.name] as UploadedFile];
        }
        (files[part.name] as UploadedFile[]).push(file);
      } else {
        files[part.name || "file"] = file;
      }
    } else {
      // 普通表单字段
      const decoder = new TextDecoder();
      const value = decoder.decode(part.data);
      if (part.name) {
        fields[part.name] = value;
      }
    }
  }

  return { fields, files };
}

/**
 * multipart 数据部分
 */
interface MultipartPart {
  name?: string;
  filename?: string;
  type: string;
  data: Uint8Array;
}

/**
 * 解析 multipart 请求体
 * @param body - 请求体
 * @param boundary - boundary 字符串
 * @returns 解析后的部分数组
 */
function parseMultipartBody(
  body: Uint8Array,
  boundary: string,
): MultipartPart[] {
  const parts: MultipartPart[] = [];

  // 将 body 转换为字符串进行解析
  const text = new TextDecoder().decode(body);
  const boundaryLine = `--${boundary}`;
  const endBoundaryLine = `--${boundary}--`;

  // 分割部分
  const lines = text.split("\r\n");
  let currentPart: Partial<MultipartPart> & { dataLines: string[] } = {
    dataLines: [],
  };
  let inHeaders = true;
  let headerText = "";

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // 开始新部分
    if (line === boundaryLine) {
      if (currentPart.dataLines.length > 0 || currentPart.name) {
        // 保存上一部分
        if (currentPart.name !== undefined || currentPart.filename) {
          // 合并数据行
          const dataString = currentPart.dataLines.join("\r\n");
          const encoder = new TextEncoder();
          const part: MultipartPart = {
            name: currentPart.name,
            filename: currentPart.filename,
            type: currentPart.type || "application/octet-stream",
            data: encoder.encode(dataString),
          };
          parts.push(part);
        }
      }

      // 开始新部分
      currentPart = { dataLines: [] };
      inHeaders = true;
      headerText = "";
      continue;
    }

    // 结束标记
    if (line === endBoundaryLine) {
      // 保存最后一部分
      if (currentPart.dataLines.length > 0 || currentPart.name) {
        const dataString = currentPart.dataLines.join("\r\n");
        const encoder = new TextEncoder();
        const part: MultipartPart = {
          name: currentPart.name,
          filename: currentPart.filename,
          type: currentPart.type || "application/octet-stream",
          data: encoder.encode(dataString),
        };
        parts.push(part);
      }
      break;
    }

    // 空行表示头部结束
    if (line === "" && inHeaders) {
      inHeaders = false;

      // 解析 Content-Disposition
      const dispMatch = headerText.match(
        /Content-Disposition: form-data; name="([^"]*)"(?:; filename="([^"]*)")?/i,
      );

      if (dispMatch) {
        currentPart.name = dispMatch[1];
        currentPart.filename = dispMatch[2] || undefined;
      }

      // 解析 Content-Type
      const typeMatch = headerText.match(/Content-Type: (.+)/i);
      currentPart.type = typeMatch ? typeMatch[1].trim() : "application/octet-stream";

      continue;
    }

    // 收集头部或数据
    if (inHeaders) {
      headerText += line + "\r\n";
    } else {
      currentPart.dataLines.push(line);
    }
  }

  return parts;
}

/**
 * 清理文件名，移除不安全字符
 * @param filename - 原始文件名
 * @returns 清理后的文件名
 */
export function sanitizeFilename(filename: string): string {
  // 移除路径分隔符和特殊字符
  const cleaned = filename.replace(/[^a-zA-Z0-9._-]/g, "_");

  // 限制长度
  if (cleaned.length > 255) {
    const ext = cleaned.includes(".")
      ? cleaned.slice(cleaned.lastIndexOf("."))
      : "";
    const nameWithoutExt = cleaned.slice(0, cleaned.lastIndexOf(".")) || cleaned;
    return nameWithoutExt.slice(0, 255 - ext.length) + ext;
  }

  return cleaned;
}

/**
 * 生成唯一文件名（使用 nanoid）
 * @param originalFilename - 原始文件名
 * @returns 唯一文件名（格式：原始名称_nanoid.扩展名）
 *
 * @example
 * generateUniqueFilename("photo.jpg") // "photo_aB1cD2eF3gH4jK5lM6nO7P.jpg"
 * generateUniqueFilename("document.pdf") // "document_xY9z8w7v6u5t4s3r2q1p.pdf"
 */
export function generateUniqueFilename(originalFilename: string): string {
  // 生成唯一的 nanoid（21 个字符）
  const uniqueId = nanoid();

  // 提取文件扩展名
  const ext = originalFilename.includes(".")
    ? originalFilename.slice(originalFilename.lastIndexOf("."))
    : "";

  // 提取文件名（不含扩展名）
  const name = originalFilename.slice(0, originalFilename.lastIndexOf(".")) ||
    originalFilename;

  // 返回格式：原始名称_nanoid.扩展名
  // 如果原始名称太长，可以截断
  const maxNameLength = 50; // 限制原始名称长度
  const truncatedName = name.length > maxNameLength
    ? name.slice(0, maxNameLength)
    : name;

  return `${truncatedName}_${uniqueId}${ext}`;
}
