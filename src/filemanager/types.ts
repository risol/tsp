/**
 * 文件管理器类型定义
 */

/**
 * 文件管理器配置接口
 */
export interface FileManagerConfig {
  /** 是否启用（默认 false） */
  enabled?: boolean;
  /** 访问路径（默认 "/__filemanager"） */
  path?: string;
  /** 访问密码（必需） */
  password: string;
  /** 是否允许访问 root 外（默认 false） */
  allowOutsideRoot?: boolean;
  /** 允许的路径白名单 */
  allowedPaths?: string[];
  /** 禁止的路径黑名单 */
  deniedPaths?: string[];
  /** 最大上传大小（字节，默认 100MB） */
  maxUploadSize?: number;
  /** 允许上传的扩展名 */
  allowedExtensions?: string[];
  /** 禁止上传的扩展名 */
  deniedExtensions?: string[];
  /** 是否允许删除 */
  allowDelete?: boolean;
  /** 是否允许重命名 */
  allowRename?: boolean;
  /** 是否允许创建目录 */
  allowMkdir?: boolean;
  /** 是否允许移动 */
  allowMove?: boolean;
  /** 是否允许解压（默认 true） */
  allowExtract?: boolean;
  /** 是否允许压缩（默认 true） */
  allowCompress?: boolean;
  /** 允许的压缩文件格式（默认 ["zip", "tar", "tgz"]） */
  allowedArchiveExtensions?: ArchiveType[] | null;
  /** 最大解压文件大小（字节，默认 1GB） */
  maxExtractSize?: number;
  /** 最大压缩文件大小（字节，默认 500MB） */
  maxCompressSize?: number;
  /** ZIP 炸弹防护：最大文件数（默认 10000） */
  maxExtractFileCount?: number;
}

/**
 * 文件信息接口
 */
export interface FileInfo {
  /** 文件名 */
  name: string;
  /** 是否是目录 */
  isDirectory: boolean;
  /** 文件大小（字节） */
  size: number;
  /** 修改时间 */
  modifiedTime: Date;
  /** 文件扩展名 */
  extension?: string;
}

/**
 * 目录浏览结果
 */
export interface BrowseResult {
  /** 当前路径 */
  path: string;
  /** 父级路径 */
  parentPath: string | null;
  /** 文件列表 */
  files: FileInfo[];
}

/**
 * API 响应基础类型
 */
export interface APIResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}

/**
 * 登录请求
 */
export interface LoginRequest {
  password: string;
}

/**
 * 登录响应
 */
export interface LoginResponse {
  csrfToken: string;
}

/**
 * 浏览请求
 */
export interface BrowseRequest {
  path?: string;
}

/**
 * 上传进度
 */
export interface UploadProgress {
  /** 已上传字节数 */
  loaded: number;
  /** 总字节数 */
  total: number;
  /** 进度百分比 */
  percentage: number;
}

/**
 * 重命名请求
 */
export interface RenameRequest {
  /** 源路径 */
  oldPath: string;
  /** 新名称 */
  newName: string;
}

/**
 * 移动请求
 */
export interface MoveRequest {
  /** 源路径 */
  oldPath: string;
  /** 目标路径 */
  newPath: string;
}

/**
 * 创建目录请求
 */
export interface MkdirRequest {
  /** 父目录路径 */
  parentPath: string;
  /** 目录名称 */
  dirName: string;
}

/**
 * 删除请求
 */
export interface DeleteRequest {
  /** 要删除的路径 */
  path: string;
}

/**
 * 压缩文件类型
 */
export type ArchiveType = "zip" | "tar" | "tgz";

/**
 * 解压请求
 */
export interface ExtractRequest {
  /** 压缩文件路径 */
  archivePath: string;
  /** 目标目录（可选，默认为当前目录） */
  targetDir?: string;
}

/**
 * 压缩请求
 */
export interface CompressRequest {
  /** 源文件路径列表 */
  sourcePaths: string[];
  /** 目标 ZIP 文件路径 */
  targetPath: string;
  /** 是否包含源目录本身（默认 false） */
  includeSrc?: boolean;
}

/**
 * 批量移动请求
 */
export interface BatchMoveRequest {
  /** 源文件路径列表 */
  sourcePaths: string[];
  /** 目标目录路径 */
  targetDir: string;
}
