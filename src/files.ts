/**
 * File upload handling module
 * Provides multipart/form-data parsing and file saving functionality
 */

import { join } from "std/path";
import { nanoid } from "nanoid";

/**
 * Uploaded file information
 */
export interface UploadedFile {
  /** Original file name */
  name: string;
  /** MIME type */
  type: string;
  /** File size in bytes */
  size: number;
  /** File content (Uint8Array) */
  data: Uint8Array;
  /** Save file to specified path */
  save(path: string): Promise<void>;
  /** Convert to text (for text files) */
  text(): Promise<string>;
}

/**
 * Create uploaded file object
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
     * Save file to specified path
     * @param path - Target path (can be relative or absolute)
     */
    async save(path: string): Promise<void> {
      await Deno.writeFile(path, this.data);
    },

    /**
     * Convert file content to text
     */
    async text(): Promise<string> {
      const decoder = new TextDecoder();
      return decoder.decode(this.data);
    },
  };

  return file;
}

/**
 * File upload configuration
 */
export interface FileUploadOptions {
  /** Maximum file size in bytes, default 10MB */
  maxSize?: number;
  /** Allowed file types (MIME types), all types allowed by default */
  allowedTypes?: string[];
  /** Upload directory (for default save behavior), default "./uploads" */
  uploadDir?: string;
}

/**
 * Parse multipart/form-data request body
 * @param requestBody - Request body (Uint8Array)
 * @param contentType - Content-Type header (contains boundary)
 * @param options - Upload configuration options
 * @returns Parsed form data and files
 */
export async function parseMultipartFormData(
  requestBody: Uint8Array,
  contentType: string,
  options: FileUploadOptions = {},
): Promise<{
  fields: Record<string, string>;
  files: Record<string, UploadedFile | UploadedFile[]>;
}> {
  // Extract boundary
  const boundaryMatch = contentType.match(/boundary=([^;]+)/i);
  if (!boundaryMatch) {
    throw new Error("Invalid Content-Type: missing boundary");
  }

  const boundary = boundaryMatch[1];
  const fields: Record<string, string> = {};
  const files: Record<string, UploadedFile | UploadedFile[]> = {};

  // Parse multipart data
  const parts = parseMultipartBody(requestBody, boundary);

  for (const part of parts) {
    // Check file size limit
    const maxSize = options.maxSize || 10 * 1024 * 1024; // Default 10MB
    if (part.data.byteLength > maxSize) {
      throw new Error(
        `File "${part.name || "unknown"}" exceeds maximum size of ${maxSize} bytes`,
      );
    }

    // If filename exists, it's a file upload
    if (part.filename) {
      // Check file type restriction
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

      // If it's an array field (multiple files with same name field), process as array
      if (part.name && files[part.name]) {
        // Same name file exists, convert to array
        if (!Array.isArray(files[part.name])) {
          files[part.name] = [files[part.name] as UploadedFile];
        }
        (files[part.name] as UploadedFile[]).push(file);
      } else {
        files[part.name || "file"] = file;
      }
    } else {
      // Regular form field
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
 * Multipart data part
 */
interface MultipartPart {
  name?: string;
  filename?: string;
  type: string;
  data: Uint8Array;
}

/**
 * Parse multipart request body
 * @param body - Request body
 * @param boundary - Boundary string
 * @returns Array of parsed parts
 */
function parseMultipartBody(
  body: Uint8Array,
  boundary: string,
): MultipartPart[] {
  const parts: MultipartPart[] = [];

  // Convert body to string for parsing
  const text = new TextDecoder().decode(body);
  const boundaryLine = `--${boundary}`;
  const endBoundaryLine = `--${boundary}--`;

  // Split parts
  const lines = text.split("\r\n");
  let currentPart: Partial<MultipartPart> & { dataLines: string[] } = {
    dataLines: [],
  };
  let inHeaders = true;
  let headerText = "";

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Start new part
    if (line === boundaryLine) {
      if (currentPart.dataLines.length > 0 || currentPart.name) {
        // Save previous part
        if (currentPart.name !== undefined || currentPart.filename) {
          // Merge data lines
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

      // Start new part
      currentPart = { dataLines: [] };
      inHeaders = true;
      headerText = "";
      continue;
    }

    // End marker
    if (line === endBoundaryLine) {
      // Save last part
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

    // Empty line indicates end of headers
    if (line === "" && inHeaders) {
      inHeaders = false;

      // Parse Content-Disposition
      const dispMatch = headerText.match(
        /Content-Disposition: form-data; name="([^"]*)"(?:; filename="([^"]*)")?/i,
      );

      if (dispMatch) {
        currentPart.name = dispMatch[1];
        currentPart.filename = dispMatch[2] || undefined;
      }

      // Parse Content-Type
      const typeMatch = headerText.match(/Content-Type: (.+)/i);
      currentPart.type = typeMatch ? typeMatch[1].trim() : "application/octet-stream";

      continue;
    }

    // Collect headers or data
    if (inHeaders) {
      headerText += line + "\r\n";
    } else {
      currentPart.dataLines.push(line);
    }
  }

  return parts;
}

/**
 * Sanitize filename, remove unsafe characters
 * @param filename - Original filename
 * @returns Sanitized filename
 */
export function sanitizeFilename(filename: string): string {
  // Remove path separators and special characters
  const cleaned = filename.replace(/[^a-zA-Z0-9._-]/g, "_");

  // Limit length
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
 * Generate unique filename (using nanoid)
 * @param originalFilename - Original filename
 * @returns Unique filename (format: original_name_nanoid.extension)
 *
 * @example
 * generateUniqueFilename("photo.jpg") // "photo_aB1cD2eF3gH4jK5lM6nO7P.jpg"
 * generateUniqueFilename("document.pdf") // "document_xY9z8w7v6u5t4s3r2q1p.pdf"
 */
export function generateUniqueFilename(originalFilename: string): string {
  // Generate unique nanoid (21 characters)
  const uniqueId = nanoid();

  // Extract file extension
  const ext = originalFilename.includes(".")
    ? originalFilename.slice(originalFilename.lastIndexOf("."))
    : "";

  // Extract filename (without extension)
  const name = originalFilename.slice(0, originalFilename.lastIndexOf(".")) ||
    originalFilename;

  // Return format: original_name_nanoid.extension
  // If original name is too long, truncate it
  const maxNameLength = 50; // Limit original name length
  const truncatedName = name.length > maxNameLength
    ? name.slice(0, maxNameLength)
    : name;

  return `${truncatedName}_${uniqueId}${ext}`;
}
