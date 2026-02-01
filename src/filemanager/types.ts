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
