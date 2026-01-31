/**
 * Session Management Module
 *
 * Provides secure session management with HMAC-SHA256 signed session IDs,
 * cookie-based storage, and automatic cleanup of expired sessions.
 */

// ============== Type Definitions ==============

/**
 * User information stored in session
 */
export interface SessionUser {
  id: string;
  name: string;
  email?: string;
  role?: string;
  [key: string]: unknown;
}

/**
 * Session options configuration
 */
export interface SessionOptions {
  /** Cookie name for session ID (default: 'tsp_session') */
  cookieName?: string;
  /** Session max age in seconds (default: 86400 = 1 day) */
  maxAge?: number;
  /** Cleanup interval in milliseconds (default: 300000 = 5 minutes) */
  cleanupInterval?: number;
  /** Secret key for HMAC signing (required in production) */
  secret?: Uint8Array;
  /** Set secure flag on cookie (default: true) */
  secure?: boolean;
  /** Set httpOnly flag on cookie (default: true) */
  httpOnly?: boolean;
  /** SameSite attribute (default: 'Strict') */
  sameSite?: "Strict" | "Lax" | "None";
  /** Cookie path (default: '/') */
  path?: string;
  /** Enable rolling sessions - refresh expiration on access (default: true) */
  rolling?: boolean;
  /** Auto-touch session on access (default: true) */
  autoTouch?: boolean;
}

/**
 * Internal session data structure
 */
interface SessionData {
  /** Signed session ID (rawId.signature) */
  id: string;
  /** Raw session ID (UUID) */
  rawId: string;
  /** Session data store */
  data: Map<string, unknown>;
  /** Creation timestamp */
  createdAt: number;
  /** Last access timestamp */
  lastTouched: number;
  /** Expiration timestamp */
  expiresAt: number;
  /** Whether session is destroyed */
  isDestroyed: boolean;
}

/**
 * Session Manager API
 * Provides type-safe session operations for page handlers
 */
export interface SessionManager {
  /** Get current user from session */
  getUser(): Promise<SessionUser | null>;
  /** Login user - creates new session or updates existing */
  login(userId: string, userData?: Partial<SessionUser>): Promise<void>;
  /** Logout user - destroys session */
  logout(): Promise<void>;
  /** Set session data */
  set(key: string, value: unknown): Promise<void>;
  /** Get session data */
  get<T = unknown>(key: string): Promise<T | null>;
  /** Delete session data */
  delete(key: string): Promise<void>;
  /** Regenerate session ID (prevents session fixation attacks) */
  regenerateId(): Promise<void>;
  /** Refresh session expiration time */
  touch(): Promise<void>;
  /** Check if session is valid */
  isValid(): Promise<boolean>;
  /** Get current session ID */
  getId(): string;
}

// ============== SessionStore Class ==============

/**
 * Session Store - Manages session storage and lifecycle
 *
 * This is an internal class that should be used as a singleton.
 * All sessions are stored in memory with automatic cleanup of expired sessions.
 */
class SessionStore {
  private sessions: Map<string, SessionData>;
  private secret: Uint8Array;
  private options: Required<SessionOptions>;
  private cryptoKey: CryptoKey | null = null;
  private cleanupTimer: number | null = null;

  constructor(options: SessionOptions) {
    this.sessions = new Map();
    this.options = {
      cookieName: options.cookieName || "tsp_session",
      maxAge: options.maxAge || 86400,
      cleanupInterval: options.cleanupInterval || 300000,
      secret: options.secret || this.generateSecret(),
      secure: options.secure !== false,
      httpOnly: options.httpOnly !== false,
      sameSite: options.sameSite || "Strict",
      path: options.path || "/",
      rolling: options.rolling !== false,
      autoTouch: options.autoTouch !== false,
    };

    this.secret = this.options.secret;
    this.startCleanup();
  }

  /**
   * Create a new session
   */
  async create(
    userId: string,
    userData?: Partial<SessionUser>,
  ): Promise<SessionData> {
    const rawId = crypto.randomUUID();
    const signedId = await this.signId(rawId);
    const now = Date.now();

    // Build user object without undefined properties
    const user: SessionUser = {
      id: userId,
      name: userData?.name || userId,
    };

    if (userData?.email !== undefined) {
      user.email = userData.email;
    }
    if (userData?.role !== undefined) {
      user.role = userData.role;
    }

    // Add any other custom properties
    if (userData) {
      for (const [key, value] of Object.entries(userData)) {
        if (
          key !== "id" && key !== "name" && key !== "email" && key !== "role"
        ) {
          user[key] = value;
        }
      }
    }

    const session: SessionData = {
      id: signedId,
      rawId,
      data: new Map([["user", user]]),
      createdAt: now,
      lastTouched: now,
      expiresAt: now + (this.options.maxAge * 1000),
      isDestroyed: false,
    };

    this.sessions.set(signedId, session);
    return session;
  }

  /**
   * Get session by signed ID
   * Returns null if session is invalid, expired, or destroyed
   */
  async get(signedId: string): Promise<SessionData | null> {
    const rawId = await this.verifySignedId(signedId);
    if (!rawId) {
      return null;
    }

    const session = this.sessions.get(signedId);
    if (!session || session.isDestroyed) {
      return null;
    }

    // Check expiration
    if (this.isExpired(session)) {
      this.sessions.delete(signedId);
      return null;
    }

    // Auto-touch if enabled
    if (this.options.autoTouch && this.options.rolling) {
      await this.touch(signedId);
    }

    return session;
  }

  /**
   * Save session (update existing)
   */
  async save(session: SessionData): Promise<void> {
    if (session.isDestroyed) {
      this.sessions.delete(session.id);
    } else {
      this.sessions.set(session.id, session);
    }
  }

  /**
   * Destroy session
   */
  async destroy(signedId: string): Promise<void> {
    const session = this.sessions.get(signedId);
    if (session) {
      session.isDestroyed = true;
      this.sessions.delete(signedId);
    }
  }

  /**
   * Refresh session expiration time
   */
  async touch(signedId: string): Promise<void> {
    const session = this.sessions.get(signedId);
    if (session && !session.isDestroyed) {
      session.lastTouched = Date.now();
      session.expiresAt = Date.now() + (this.options.maxAge * 1000);
    }
  }

  /**
   * Regenerate session ID
   * Returns new signed ID
   */
  async regenerateId(signedId: string): Promise<string> {
    const session = this.sessions.get(signedId);
    if (!session || session.isDestroyed) {
      throw new Error("Session not found or destroyed");
    }

    // Remove old session
    this.sessions.delete(signedId);

    // Generate new ID
    const newRawId = crypto.randomUUID();
    const newSignedId = await this.signId(newRawId);

    // Update session
    session.rawId = newRawId;
    session.id = newSignedId;

    // Save with new ID
    this.sessions.set(newSignedId, session);

    return newSignedId;
  }

  // ============== Signing Methods ==============

  /**
   * Sign session ID with HMAC-SHA256
   */
  private async signId(id: string): Promise<string> {
    const key = await this.getCryptoKey();
    const data = new TextEncoder().encode(id);
    const signature = await crypto.subtle.sign("HMAC", key, data);
    const signatureB64 = this.base64UrlEncode(new Uint8Array(signature));
    return `${id}.${signatureB64}`;
  }

  /**
   * Verify signed session ID
   * Returns raw ID if valid, null otherwise
   */
  private async verifySignedId(signedId: string): Promise<string | null> {
    const parts = signedId.split(".");
    if (parts.length !== 2) {
      return null;
    }

    const [rawId, sigB64] = parts;

    try {
      const key = await this.getCryptoKey();
      const data = new TextEncoder().encode(rawId);
      const signature = this.base64UrlDecode(sigB64);
      const isValid = await crypto.subtle.verify(
        "HMAC",
        key,
        signature.buffer as ArrayBuffer,
        data,
      );

      return isValid ? rawId : null;
    } catch {
      return null;
    }
  }

  /**
   * Get or create HMAC key
   */
  private async getCryptoKey(): Promise<CryptoKey> {
    if (!this.cryptoKey) {
      // Create a proper ArrayBuffer from Uint8Array
      const secretBuffer = new ArrayBuffer(this.secret.length);
      const secretView = new Uint8Array(secretBuffer);
      secretView.set(this.secret);

      this.cryptoKey = await crypto.subtle.importKey(
        "raw",
        secretBuffer,
        { name: "HMAC", hash: "SHA-256" },
        false,
        ["sign", "verify"],
      );
    }
    return this.cryptoKey;
  }

  /**
   * Base64 URL encode (URL-safe variant)
   */
  private base64UrlEncode(data: Uint8Array): string {
    const base64 = btoa(String.fromCharCode(...data));
    return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
  }

  /**
   * Base64 URL decode
   */
  private base64UrlDecode(str: string): Uint8Array {
    let base64 = str.replace(/-/g, "+").replace(/_/g, "/");
    while (base64.length % 4) {
      base64 += "=";
    }
    const binary = atob(base64);
    return new Uint8Array([...binary].map((c) => c.charCodeAt(0)));
  }

  /**
   * Generate random secret (for development only)
   */
  private generateSecret(): Uint8Array {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);
    return bytes;
  }

  // ============== Cleanup Methods ==============

  /**
   * Check if session is expired
   */
  private isExpired(session: SessionData): boolean {
    return Date.now() >= session.expiresAt;
  }

  /**
   * Clean up expired sessions
   */
  private cleanupExpiredSessions(): void {
    const now = Date.now();
    let cleanedCount = 0;

    for (const [signedId, session] of this.sessions.entries()) {
      if (session.isDestroyed || now >= session.expiresAt) {
        this.sessions.delete(signedId);
        cleanedCount++;
      }
    }

    if (cleanedCount > 0) {
      console.log(`[Session] Cleaned up ${cleanedCount} expired session(s)`);
    }
  }

  /**
   * Start automatic cleanup timer
   */
  private startCleanup(): void {
    this.cleanupTimer = setInterval(() => {
      this.cleanupExpiredSessions();
    }, this.options.cleanupInterval);
  }

  /**
   * Stop cleanup timer
   */
  private stopCleanup(): void {
    if (this.cleanupTimer !== null) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = null;
    }
  }

  /**
   * Get options
   */
  getOptions(): Required<SessionOptions> {
    return this.options;
  }

  /**
   * Cleanup and destroy store (for testing)
   */
  cleanupStore(): void {
    this.stopCleanup();
    this.sessions.clear();
  }
}

// ============== SessionManager Factory ==============

/**
 * Create a SessionManager instance for the current request
 */
export function createSessionManager(
  ctx: {
    method: string;
    url: URL;
    headers: Headers;
    query: Record<string, string>;
    body: unknown;
    cookies: Record<string, string> | {
      get(name: string): string | undefined;
      set(name: string, value: string, options: unknown): void;
      delete(name: string, options?: unknown): void;
    };
    file: string;
    root: string;
  },
  store: SessionStore,
  cookieManager?: {
    set(name: string, value: string, options: unknown): void;
    delete(name: string, options?: unknown): void;
  },
): SessionManager {
  const options = store.getOptions();
  const cookieName = options.cookieName;

  // Get session ID from cookies (handle both Record and CookieManager)
  const cookiesRecord = ctx.cookies as Record<string, string>;
  const signedId = cookiesRecord[cookieName] || "";

  // 🔧 Mutable session variable that gets updated after login/logout
  // Initially null, loaded on first access
  let currentSession: SessionData | null = null;
  let sessionLoaded = false;

  // Function to get or load the session
  async function getSession(): Promise<SessionData | null> {
    if (!sessionLoaded) {
      currentSession = signedId ? await store.get(signedId) : null;
      sessionLoaded = true;
    }
    return currentSession;
  }

  // Use cookieManager if provided
  const cookies = cookieManager;

  const manager: SessionManager = {
    async getUser(): Promise<SessionUser | null> {
      const session = await getSession();
      if (!session) {
        return null;
      }
      return session.data.get("user") as SessionUser || null;
    },

    async login(
      userId: string,
      userData?: Partial<SessionUser>,
    ): Promise<void> {
      let session = await getSession();

      // If session exists, update it
      if (session) {
        const user: SessionUser = {
          id: userId,
          name: userData?.name || userId,
          email: userData?.email,
          role: userData?.role,
          ...userData,
        };
        session.data.set("user", user);
        await store.save(session);
      } else {
        // Create new session
        session = await store.create(userId, userData);
        // Update the mutable variable
        currentSession = session;
      }

      // Set cookie using CookieManager
      if (cookies) {
        cookies.set(cookieName, session.id, {
          httpOnly: options.httpOnly,
          secure: options.secure,
          sameSite: options.sameSite,
          path: options.path,
          maxAge: options.maxAge,
        });
      }
    },

    async logout(): Promise<void> {
      const session = await getSession();
      if (session) {
        await store.destroy(session.id);
        if (cookies) {
          cookies.delete(cookieName, { path: options.path });
        }
        // Update the mutable variable
        currentSession = null;
      }
    },

    async set(key: string, value: unknown): Promise<void> {
      const session = await getSession();
      if (!session) {
        throw new Error("No active session");
      }
      session.data.set(key, value);
      await store.save(session);
    },

    async get<T = unknown>(key: string): Promise<T | null> {
      const session = await getSession();
      if (!session) {
        return null;
      }
      const value = session.data.get(key);
      return value as T || null;
    },

    async delete(key: string): Promise<void> {
      const session = await getSession();
      if (!session) {
        return;
      }
      session.data.delete(key);
      await store.save(session);
    },

    async regenerateId(): Promise<void> {
      const session = await getSession();
      if (!session) {
        return;
      }
      const newId = await store.regenerateId(session.id);
      if (cookies) {
        cookies.set(cookieName, newId, {
          httpOnly: options.httpOnly,
          secure: options.secure,
          sameSite: options.sameSite,
          path: options.path,
          maxAge: options.maxAge,
        });
      }
    },

    async touch(): Promise<void> {
      const session = await getSession();
      if (!session) {
        return;
      }
      await store.touch(session.id);
    },

    async isValid(): Promise<boolean> {
      const session = await getSession();
      return session !== null && !session.isDestroyed;
    },

    getId(): string {
      // Return the signed session ID from the cookie
      // This is synchronous because we already have the signedId
      return signedId || "";
    },
  };

  return manager;
}

// ============== Default Options ==============

/**
 * Get default session options
 */
export function getDefaultOptions(): SessionOptions {
  return {
    cookieName: "tsp_session",
    maxAge: 86400, // 1 day
    cleanupInterval: 300000, // 5 minutes
    secure: true,
    httpOnly: true,
    sameSite: "Strict",
    path: "/",
    rolling: true,
    autoTouch: true,
  };
}

// ============== Export SessionStore for main.ts ==============

export { SessionStore };
