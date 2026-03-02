/**
 * File manager authentication module
 * Provides password hashing, verification, session management and CSRF Token functionality
 */

import { nanoid } from "nanoid";

// ============== Constants ==============

const SESSION_KEY = "fm_session";
const CSRF_SESSION_KEY = "fm_csrf";
const HASH_ITERATIONS = 100000;
const HASH_KEY_LENGTH = 32;

// ============== Password Hashing ==============

/**
 * Hash password using PBKDF2
 * @param password Plain text password
 * @param salt Salt (Base64 encoded)
 * @returns Hashed password (Base64 encoded)
 */
export async function hashPassword(
  password: string,
  salt?: string,
): Promise<{ hash: string; salt: string }> {
  // If no salt provided, generate new one
  const saltArray = new Uint8Array(HASH_KEY_LENGTH);
  const saltBuffer = salt
    ? base64ToUint8Array(salt)
    : crypto.getRandomValues(saltArray);

  // Encode password
  const passwordBuffer = new TextEncoder().encode(password);

  // Import key material
  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    passwordBuffer,
    "PBKDF2",
    false,
    ["deriveBits"],
  );

  // Derive key (using standard ArrayBuffer)
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
    HASH_KEY_LENGTH * 8, // Convert to bits
  );

  const hashArray = new Uint8Array(derivedBits);
  const hashBase64 = uint8ArrayToBase64(hashArray);
  const saltBase64 = uint8ArrayToBase64(saltBuffer);

  return { hash: hashBase64, salt: saltBase64 };
}

/**
 * Verify password
 * @param password Plain text password
 * @param hash Hash value (Base64 encoded)
 * @param salt Salt (Base64 encoded)
 * @returns Whether it matches
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
 * Constant time string comparison (prevent timing attacks)
 * @param a String a
 * @param b String b
 * @returns Whether equal
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

// ============== Base64 Encoding/Decoding ==============

/**
 * Uint8Array to Base64
 */
function uint8ArrayToBase64(array: Uint8Array): string {
  let binary = "";
  for (const byte of array) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

/**
 * Base64 to Uint8Array
 */
function base64ToUint8Array(base64: string): Uint8Array {
  const binary = atob(base64);
  const array = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    array[i] = binary.charCodeAt(i);
  }
  return array;
}

// ============== Session Management ==============

/**
 * Session data interface
 */
interface FMSessionData {
  authenticated: boolean;
  csrfToken: string;
  createdAt: number;
  lastTouched: number;
}

/**
 * Session storage (in-memory)
 */
const sessionStore = new Map<string, FMSessionData>();

/**
 * Session expiry time (2 hours)
 */
const SESSION_EXPIRY = 2 * 60 * 60 * 1000;

/**
 * Create new session
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
 * Validate session
 * @param sessionId Session ID
 * @returns Whether valid
 */
export function validateSession(sessionId: string): boolean {
  const sessionData = sessionStore.get(sessionId);

  if (!sessionData) {
    return false;
  }

  // Check if expired
  const now = Date.now();
  if (now - sessionData.lastTouched > SESSION_EXPIRY) {
    sessionStore.delete(sessionId);
    return false;
  }

  // Update last access time
  sessionData.lastTouched = now;

  return sessionData.authenticated;
}

/**
 * Destroy session
 * @param sessionId Session ID
 */
export function destroySession(sessionId: string): void {
  sessionStore.delete(sessionId);
}

/**
 * Get CSRF Token from session
 * @param sessionId Session ID
 * @returns CSRF Token or null
 */
export function getCSRFToken(sessionId: string): string | null {
  const sessionData = sessionStore.get(sessionId);
  return sessionData?.csrfToken || null;
}

/**
 * Clean up expired sessions
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
    console.log(`[File Manager] Cleaned up ${cleanedCount} expired sessions`);
  }
}

// Start scheduled cleanup (every 5 minutes)
setInterval(cleanupExpiredSessions, 5 * 60 * 1000);

// ============== CSRF Token ==============

/**
 * Generate CSRF Token
 * @returns CSRF Token
 */
function generateCSRFToken(): string {
  // Use nanoid to generate 32-character token
  return nanoid(32);
}

/**
 * Verify CSRF Token
 * @param sessionId Session ID
 * @param token CSRF Token
 * @returns Whether valid
 */
export function verifyCSRFToken(sessionId: string, token: string): boolean {
  const sessionData = sessionStore.get(sessionId);

  if (!sessionData) {
    return false;
  }

  return constantTimeCompare(sessionData.csrfToken, token);
}

// ============== Cookie Helper Functions ==============

/**
 * Get Session ID from request
 * @param cookies Cookie object
 * @returns Session ID or empty string
 */
export function getSessionIdFromCookies(
  cookies: Record<string, string>,
): string {
  return cookies[SESSION_KEY] || "";
}

/**
 * Set Session Cookie response header
 * @param sessionId Session ID
 * @param secure Whether to add Secure attribute (default true)
 * @returns Set-Cookie header value
 */
export function createSessionCookieHeader(
  sessionId: string,
  secure: boolean = true,
): string {
  const maxAge = 2 * 60 * 60; // 2 hours

  const cookieParts = [
    `${SESSION_KEY}=${sessionId}`,
    `Max-Age=${maxAge}`,
    "Path=/",
    "HttpOnly",
    "SameSite=Strict",
  ];

  // Decide whether to add Secure flag based on config
  if (secure) {
    cookieParts.push("Secure");
  }

  return cookieParts.join("; ");
}

/**
 * Delete Session Cookie response header
 * @param secure Whether to add Secure attribute (must match when setting)
 * @returns Set-Cookie header value
 */
export function createClearSessionCookieHeader(secure: boolean = true): string {
  const cookieParts = [
    `${SESSION_KEY}=`,
    "Max-Age=0",
    "Path=/",
    "HttpOnly",
    "SameSite=Strict",
  ];

  // Decide whether to add Secure flag based on config (must match when setting)
  if (secure) {
    cookieParts.push("Secure");
  }

  return cookieParts.join("; ");
}
