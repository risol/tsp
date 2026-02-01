/**
 * 文件管理器认证模块
 * 提供密码哈希、验证、session 管理和 CSRF Token 功能
 */

import { nanoid } from "nanoid";

// ============== 常量 ==============

const SESSION_KEY = "fm_session";
const CSRF_SESSION_KEY = "fm_csrf";
const HASH_ITERATIONS = 100000;
const HASH_KEY_LENGTH = 32;

// ============== 密码哈希 ==============

/**
 * 使用 PBKDF2 哈希密码
 * @param password 明文密码
 * @param salt 盐值（Base64 编码）
 * @returns 哈希后的密码（Base64 编码）
 */
export async function hashPassword(
  password: string,
  salt?: string,
): Promise<{ hash: string; salt: string }> {
  // 如果没有提供盐值，生成新的
  const saltArray = new Uint8Array(HASH_KEY_LENGTH);
  const saltBuffer = salt
    ? base64ToUint8Array(salt)
    : crypto.getRandomValues(saltArray);

  // 编码密码
  const passwordBuffer = new TextEncoder().encode(password);

  // 导入密钥材料
  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    passwordBuffer,
    "PBKDF2",
    false,
    ["deriveBits"],
  );

  // 派生密钥（使用标准的 ArrayBuffer）
  const saltArrayBuffer = new ArrayBuffer(saltBuffer.byteLength);
  const saltArrayView = new Uint8Array(saltArrayBuffer);
  saltArrayView.set(saltBuffer);

  const derivedBits = await crypto.subtle.deriveBits(
    {
      name: "PBKDF2",
      salt: saltArrayBuffer,
      iterations: HASH_ITERATIONS,
      hash: "SHA-256",
    },
    keyMaterial,
    HASH_KEY_LENGTH * 8, // 转换为比特
  );

  const hashArray = new Uint8Array(derivedBits);
  const hashBase64 = uint8ArrayToBase64(hashArray);
  const saltBase64 = uint8ArrayToBase64(saltBuffer);

  return { hash: hashBase64, salt: saltBase64 };
}

/**
 * 验证密码
 * @param password 明文密码
 * @param hash 哈希值（Base64 编码）
 * @param salt 盐值（Base64 编码）
 * @returns 是否匹配
 */
export async function verifyPassword(
  password: string,
  hash: string,
  salt: string,
): Promise<boolean> {
  const { hash: computedHash } = await hashPassword(password, salt);
  return constantTimeCompare(hash, computedHash);
}

/**
 * 常量时间字符串比较（防止时序攻击）
 * @param a 字符串 a
 * @param b 字符串 b
 * @returns 是否相等
 */
function constantTimeCompare(a: string, b: string): boolean {
  if (a.length !== b.length) {
    return false;
  }

  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }

  return result === 0;
}

// ============== Base64 编解码 ==============

/**
 * Uint8Array 转 Base64
 */
function uint8ArrayToBase64(array: Uint8Array): string {
  let binary = "";
  for (const byte of array) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

/**
 * Base64 转 Uint8Array
 */
function base64ToUint8Array(base64: string): Uint8Array {
  const binary = atob(base64);
  const array = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    array[i] = binary.charCodeAt(i);
  }
  return array;
}

// ============== Session 管理 ==============

/**
 * Session 数据接口
 */
interface FMSessionData {
  authenticated: boolean;
  csrfToken: string;
  createdAt: number;
  lastTouched: number;
}

/**
 * Session 存储（内存）
 */
const sessionStore = new Map<string, FMSessionData>();

/**
 * Session 过期时间（2 小时）
 */
const SESSION_EXPIRY = 2 * 60 * 60 * 1000;

/**
 * 创建新的 session
 * @returns Session ID
 */
export function createSession(): string {
  const sessionId = nanoid();
  const csrfToken = generateCSRFToken();

  const sessionData: FMSessionData = {
    authenticated: true,
    csrfToken,
    createdAt: Date.now(),
    lastTouched: Date.now(),
  };

  sessionStore.set(sessionId, sessionData);

  return sessionId;
}

/**
 * 验证 session
 * @param sessionId Session ID
 * @returns 是否有效
 */
export function validateSession(sessionId: string): boolean {
  const sessionData = sessionStore.get(sessionId);

  if (!sessionData) {
    return false;
  }

  // 检查是否过期
  const now = Date.now();
  if (now - sessionData.lastTouched > SESSION_EXPIRY) {
    sessionStore.delete(sessionId);
    return false;
  }

  // 更新最后访问时间
  sessionData.lastTouched = now;

  return sessionData.authenticated;
}

/**
 * 销毁 session
 * @param sessionId Session ID
 */
export function destroySession(sessionId: string): void {
  sessionStore.delete(sessionId);
}

/**
 * 获取 session 中的 CSRF Token
 * @param sessionId Session ID
 * @returns CSRF Token 或 null
 */
export function getCSRFToken(sessionId: string): string | null {
  const sessionData = sessionStore.get(sessionId);
  return sessionData?.csrfToken || null;
}

/**
 * 清理过期的 session
 */
export function cleanupExpiredSessions(): void {
  const now = Date.now();
  let cleanedCount = 0;

  for (const [sessionId, sessionData] of sessionStore.entries()) {
    if (now - sessionData.lastTouched > SESSION_EXPIRY) {
      sessionStore.delete(sessionId);
      cleanedCount++;
    }
  }

  if (cleanedCount > 0) {
    console.log(`[文件管理器] 清理了 ${cleanedCount} 个过期 session`);
  }
}

// 启动定时清理（每 5 分钟）
setInterval(cleanupExpiredSessions, 5 * 60 * 1000);

// ============== CSRF Token ==============

/**
 * 生成 CSRF Token
 * @returns CSRF Token
 */
function generateCSRFToken(): string {
  // 使用 nanoid 生成 32 字符长的 token
  return nanoid(32);
}

/**
 * 验证 CSRF Token
 * @param sessionId Session ID
 * @param token CSRF Token
 * @returns 是否有效
 */
export function verifyCSRFToken(sessionId: string, token: string): boolean {
  const sessionData = sessionStore.get(sessionId);

  if (!sessionData) {
    return false;
  }

  return constantTimeCompare(sessionData.csrfToken, token);
}

// ============== Cookie 辅助函数 ==============

/**
 * 获取请求中的 Session ID
 * @param cookies Cookie 对象
 * @returns Session ID 或空字符串
 */
export function getSessionIdFromCookies(
  cookies: Record<string, string>,
): string {
  return cookies[SESSION_KEY] || "";
}

/**
 * 设置 Session Cookie 的响应头
 * @param sessionId Session ID
 * @returns Set-Cookie 头的值
 */
export function createSessionCookieHeader(sessionId: string): string {
  const maxAge = 2 * 60 * 60; // 2 小时

  const cookieParts = [
    `${SESSION_KEY}=${sessionId}`,
    `Max-Age=${maxAge}`,
    "Path=/",
    "HttpOnly",
    "SameSite=Strict",
  ];

  // 在 HTTPS 环境下添加 Secure 标志
  // 注意：这里无法判断当前是否是 HTTPS，由调用方决定是否添加

  return cookieParts.join("; ");
}

/**
 * 删除 Session Cookie 的响应头
 * @returns Set-Cookie 头的值
 */
export function createClearSessionCookieHeader(): string {
  return [
    `${SESSION_KEY}=`,
    "Max-Age=0",
    "Path=/",
    "HttpOnly",
    "SameSite=Strict",
  ].join("; ");
}
