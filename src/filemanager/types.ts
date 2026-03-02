/**
 * File manager type definitions
 */

/**
 * File manager configuration interface
 */
export interface FileManagerConfig {
  /** Whether to enable (default false) */
  enabled?: boolean;
  /** Access path (default "/__filemanager") */
  path?: string;
  /** Access password (required) */
  password: string;
  /** Whether to allow access outside root (default false) */
  allowOutsideRoot?: boolean;
  /** Allowed path whitelist */
  allowedPaths?: string[];
  /** Denied path blacklist */
  deniedPaths?: string[];
  /** Max upload size (bytes, default 100MB) */
  maxUploadSize?: number;
  /** Allowed upload extensions */
  allowedExtensions?: string[];
  /** Denied upload extensions */
  deniedExtensions?: string[];
  /** Whether to allow delete */
  allowDelete?: boolean;
  /** Whether to allow rename */
  allowRename?: boolean;
  /** Whether to allow create directory */
  allowMkdir?: boolean;
  /** Whether to allow move */
  allowMove?: boolean;
  /** Whether to allow extract (default true) */
  allowExtract?: boolean;
  /** Whether to allow compress (default true) */
  allowCompress?: boolean;
  /** Allowed archive file formats (default ["zip", "tar", "tgz"]) */
  allowedArchiveExtensions?: ArchiveType[] | null;
  /** Max extract file size (bytes, default 1GB) */
  maxExtractSize?: number;
  /** Max compress file size (bytes, default 500MB) */
  maxCompressSize?: number;
  /** ZIP bomb protection: max file count (default 10000) */
  maxExtractFileCount?: number;
}

/**
 * File info interface
 */
export interface FileInfo {
  /** File name */
  name: string;
  /** Whether is directory */
  isDirectory: boolean;
  /** File size (bytes) */
  size: number;
  /** Modified time */
  modifiedTime: Date;
  /** File extension */
  extension?: string;
}

/**
 * Directory browse result
 */
export interface BrowseResult {
  /** Current path */
  path: string;
  /** Parent path */
  parentPath: string | null;
  /** File list */
  files: FileInfo[];
}

/**
 * API response base type
 */
export interface APIResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}

/**
 * Login request
 */
export interface LoginRequest {
  password: string;
}

/**
 * Login response
 */
export interface LoginResponse {
  csrfToken: string;
}

/**
 * Browse request
 */
export interface BrowseRequest {
  path?: string;
}

/**
 * Upload progress
 */
export interface UploadProgress {
  /** Bytes uploaded */
  loaded: number;
  /** Total bytes */
  total: number;
  /** Progress percentage */
  percentage: number;
}

/**
 * Rename request
 */
export interface RenameRequest {
  /** Source path */
  oldPath: string;
  /** New name */
  newName: string;
}

/**
 * Move request
 */
export interface MoveRequest {
  /** Source path */
  oldPath: string;
  /** Target path */
  newPath: string;
}

/**
 * Create directory request
 */
export interface MkdirRequest {
  /** Parent directory path */
  parentPath: string;
  /** Directory name */
  dirName: string;
}

/**
 * Delete request
 */
export interface DeleteRequest {
  /** Path to delete */
  path: string;
}

/**
 * Archive file type
 */
export type ArchiveType = "zip" | "tar" | "tgz";

/**
 * Extract request
 */
export interface ExtractRequest {
  /** Archive file path */
  archivePath: string;
  /** Target directory (optional, default is current directory) */
  targetDir?: string;
}

/**
 * Compress request
 */
export interface CompressRequest {
  /** Source file path list */
  sourcePaths: string[];
  /** Target ZIP file path */
  targetPath: string;
  /** Whether to include source directory itself (default false) */
  includeSrc?: boolean;
}

/**
 * Batch move request
 */
export interface BatchMoveRequest {
  /** Source file path list */
  sourcePaths: string[];
  /** Target directory path */
  targetDir: string;
}
