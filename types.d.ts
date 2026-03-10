/**
 * 全局类型声明
 * 只保留 JSX 必需的类型（React、JSX）和 Zod 类型
 * 所有业务类型通过 types 依赖注入提供
 */

declare global {
  /**
   * HTML 元素属性接口（兼容 React HTMLAttributes）
   */
  interface HTMLAttributesBase {
    class?: string;
    className?: string;
    style?: string | Record<string, string>;
    id?: string;
    key?: string | number;
    ref?: any;
    [key: string]: any;
  }

  /**
   * Script component - preserves script content without escaping
   * Use this instead of <script> to avoid React escaping </script> tags
   */
  function Script(props: { children?: string } & HTMLAttributesBase): any;

  /**
   * Zod 类型别名（全局可用）
   * 用于 Schema-first API 的类型定义
   */
  type ZodTypeAny = import("zod").ZodTypeAny;
  type ZodInfer<T extends import("zod").ZodTypeAny> = import("zod").infer<T>;
  type ZodObject<T extends Record<string, import("zod").ZodTypeAny>> = import("zod").ZodObject<T>;
  type ZodNumber = import("zod").ZodNumber;
  type ZodString = import("zod").ZodString;
  type ZodBoolean = import("zod").ZodBoolean;
  type ZodOptional<T extends import("zod").ZodTypeAny> = import("zod").ZodOptional<T>;
  type ZodArray<T extends import("zod").ZodTypeAny> = import("zod").ZodArray<T>;

  /**
   * JSX 类型声明
   */
  namespace JSX {
    /**
     * 内置元素类型（HTML 标签）
     */
    interface IntrinsicElements {
      [elemName: string]: any;
    }

    /**
     * element children 属性
     */
    interface ElementChildrenAttribute {
      children?: any;
    }

    /**
     * 元素属性类型
     */
    interface ElementAttributesProperty {
      props?: any;
    }
  }

  // ============================================
  // 业务类型定义（在 declare global 中，保持全局可用）
  // ============================================
type HttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "PATCH"
  | "DELETE"
  | "HEAD"
  | "OPTIONS";

/**
 * 页面上下文类型
 * 每个 TSX 页面函数接收此上下文作为参数
 * 注意：query 和 body 已移除，请使用依赖注入的 deps.query() 和 deps.body()
 */
interface PageContext {
    /** HTTP 请求方法 */
    readonly method: HttpMethod;
    /** 完整的请求 URL 对象 */
    readonly url: URL;
    /** 请求头对象 */
    readonly headers: Headers;
    /** Cookie 数据 */
    readonly cookies: Record<string, string>;
    /** 上传的文件（multipart/form-data）
     * - 单个文件：UploadedFile
     * - 多个同名文件：UploadedFile[]
     */
    readonly files: Record<string, UploadedFile | UploadedFile[]>;
    /** 当前 TSX 页面文件的路径 */
    readonly file: string;
    /** 文档根目录路径 */
    readonly root: string;
  }

  /**
   * 重定向结果
   * 页面返回此对象将触发 HTTP 重定向
   */
  interface RedirectResult {
    /** 重定向的目标 URL */
    redirect: string;
    /** 重定向状态码，默认 302 */
    status?: 301 | 302 | 303 | 307 | 308;
  }

  /**
   * 上传的文件接口
   * 表示通过 multipart/form-data 上传的文件
   */
  interface UploadedFile {
    /** 原始文件名 */
    readonly name: string;
    /** MIME 类型 */
    readonly type: string;
    /** 文件大小（字节） */
    readonly size: number;
    /** 文件内容（Uint8Array） */
    readonly data: Uint8Array;

    /**
     * 保存文件到指定路径
     * @param path - 目标路径（可以是相对路径或绝对路径）
     */
    save(path: string): Promise<void>;

    /**
     * 将文件内容转换为文本（适用于文本文件）
     */
    text(): Promise<string>;
  }

  /**
   * Response 辅助器接口
   * 提供便捷的 HTTP 响应创建方法
   */
  interface ResponseHelper {
    /**
     * 返回 JSON 响应
     * @param data - 要序列化的数据
     * @param status - HTTP 状态码，默认 200
     * @param headers - 额外的响应头
     */
    json<T = unknown>(data: T, status?: number, headers?: HeadersInit): Response;

    /**
     * 返回纯文本响应
     * @param content - 文本内容
     * @param status - HTTP 状态码，默认 200
     * @param headers - 额外的响应头
     */
    text(content: string, status?: number, headers?: HeadersInit): Response;

    /**
     * 返回 HTML 响应
     * @param content - HTML 内容
     * @param status - HTTP 状态码，默认 200
     * @param headers - 额外的响应头
     */
    html(content: string, status?: number, headers?: HeadersInit): Response;

    /**
     * 返回重定向
     * @param url - 重定向目标 URL
     * @param status - 重定向状态码，默认 302
     */
    redirect(url: string, status?: 301 | 302 | 303 | 307 | 308): RedirectResult;

    /**
     * 返回错误响应
     * @param message - 错误消息
     * @param status - HTTP 状态码，默认 500
     * @param headers - 额外的响应头
     */
    error(message: string, status?: number, headers?: HeadersInit): Response;

    /**
     * 返回文件下载响应
     * @param content - 文件内容（字符串或二进制）
     * @param filename - 文件名
     * @param headers - 额外的响应头
     */
    file(content: string | Uint8Array, filename: string, headers?: HeadersInit): Response;

    /**
     * 返回 204 No Content 响应
     */
    noContent(): Response;

    /**
     * 返回自定义 Response
     * @param body - 响应体
     * @param init - Response 初始化选项
     */
    custom(body?: BodyInit | null, init?: ResponseInit): Response;
  }

  /**
   * 日志记录器接口
   * 提供结构化的日志记录功能
   */
  interface Logger {
    /**
     * 调试日志
     * @param args - 要记录的消息内容
     */
    debug(...args: unknown[]): void;

    /**
     * 信息日志
     * @param args - 要记录的消息内容
     */
    info(...args: unknown[]): void;

    /**
     * 警告日志
     * @param args - 要记录的消息内容
     */
    warn(...args: unknown[]): void;

    /**
     * 错误日志
     * @param args - 要记录的消息内容
     */
    error(...args: unknown[]): void;
  }

  /**
   * 日志归档配置
   */
  interface LogRotationConfig {
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
   * 日志配置
   */
  interface LoggerConfig {
    /** 最小日志级别 */
    minLevel: "DEBUG" | "INFO" | "WARN" | "ERROR";
    /** 是否启用彩色输出 */
    colorize?: boolean;
    /** 是否输出到控制台 */
    console?: boolean;
    /** 日志文件路径 */
    file?: string;
    /** 日志归档配置 */
    rotation?: LogRotationConfig;
  }

  /**
   * 文件管理器配置
   */
  interface FileManagerConfig {
    /** 是否启用（默认 false） */
    enabled?: boolean;
    /** 访问路径（默认 "/__filemanager"） */
    path?: string;
    /** 访问密码（至少 6 个字符） */
    password?: string;
    /** 是否允许访问根目录外的文件（默认 false） */
    allowOutsideRoot?: boolean;
    /** 禁止访问的路径列表 */
    deniedPaths?: string[];
    /** 最大上传文件大小（字节） */
    maxUploadSize?: number;
    /** 是否允许删除文件 */
    allowDelete?: boolean;
    /** 是否允许重命名文件 */
    allowRename?: boolean;
    /** 是否允许创建目录 */
    allowMkdir?: boolean;
    /** 是否允许移动文件 */
    allowMove?: boolean;
    /** 是否允许解压文件 */
    allowExtract?: boolean;
    /** 是否允许压缩文件 */
    allowCompress?: boolean;
  }

  /**
   * 测试结果类型
   */
  interface TestResult {
    /** 测试名称 */
    name: string;
    /** 是否通过 */
    passed: boolean;
    /** 错误信息 */
    error?: string;
    /** 测试数据 */
    data?: unknown;
    /** 执行时间（毫秒） */
    duration?: number;
  }

  /**
   * 测试套件结果
   */
  interface TestSuiteResult {
    /** 总测试数 */
    total: number;
    /** 通过数 */
    passed: number;
    /** 失败数 */
    failed: number;
    /** 测试结果列表 */
    results: TestResult[];
    /** 总执行时间（毫秒） */
    duration: number;
  }

  /**
   * 模拟的 PageContext
   * 用于测试时模拟 HTTP 请求上下文
   */
  interface MockPageContext {
    /** HTTP 方法 */
    method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
    /** 请求 URL（自动转换为 URL 对象） */
    url: string | URL;
    /** 请求头 */
    headers?: Record<string, string>;
    /** URL 查询参数 */
    query?: Record<string, string>;
    /** 请求体数据 */
    body?: unknown;
    /** Cookies */
    cookies?: Record<string, string>;
  }

  /**
   * 模拟的 Response 对象
   */
  interface MockResponse {
    /** 返回 JSON 响应 */
    json(data: unknown, status?: number): Response;
    /** 返回 HTML 响应 */
    html(html: string, status?: number): Response;
    /** 返回文本响应 */
    text(text: string, status?: number): Response;
    /** 返回重定向 */
    redirect(url: string, status?: 301 | 302 | 303 | 307 | 308): Response;
  }

  /**
   * HTTP 请求模拟选项
   */
  interface MockFetchOptions {
    /** 响应数据 */
    response: unknown;
    /** 响应状态码 */
    status?: number;
    /** 响应头 */
    headers?: Record<string, string>;
    /** 响应延迟（毫秒） */
    delay?: number;
  }

  /**
   * TSP 测试辅助器接口
   */
  interface TestHelper {
    /**
     * 运行单个测试
     * @param name - 测试名称
     * @param fn - 测试函数（可以是同步或异步）
     */
    test(name: string, fn: () => void | Promise<void>): Promise<TestResult>;

    /**
     * 断言相等
     * @param actual - 实际值
     * @param expected - 期望值
     * @param message - 错误信息
     */
    assertEqual(actual: unknown, expected: unknown, message?: string): void;

    /**
     * 断言不相等
     * @param actual - 实际值
     * @param expected - 期望值
     * @param message - 错误信息
     */
    assertNotEqual(actual: unknown, expected: unknown, message?: string): void;

    /**
     * 断言为真
     * @param value - 值
     * @param message - 错误信息
     */
    assertTrue(value: unknown, message?: string): void;

    /**
     * 断言为假
     * @param value - 值
     * @param message - 错误信息
     */
    assertFalse(value: unknown, message?: string): void;

    /**
     * 断言为 null
     * @param value - 值
     * @param message - 错误信息
     */
    assertNull(value: unknown, message?: string): void;

    /**
     * 断言不为 null
     * @param value - 值
     * @param message - 错误信息
     */
    assertNotNull(value: unknown, message?: string): void;

    /**
     * 断言包含
     * @param haystack - 字符串或数组
     * @param needle - 要查找的值
     * @param message - 错误信息
     */
    assertContains(haystack: string | unknown[], needle: unknown, message?: string): void;

    /**
     * 断言抛出异常
     * @param fn - 函数
     * @param expectedError - 期望的错误类型或消息
     * @param message - 错误信息
     */
    assertThrows(fn: () => void, expectedError?: string | RegExp, message?: string): void;

    /**
     * 模拟 PageContext
     */
    mockContext(ctx: MockPageContext): PageContext;

    /**
     * 直接调用页面函数（单元测试）
     */
    runPage(pageFn: (ctx: PageContext, deps: any) => Promise<unknown>, ctx: PageContext): Promise<unknown>;

    /**
     * Mock HTTP 请求
     */
    mockFetch(urlPattern: string | RegExp, options: MockFetchOptions): void;

    /**
     * 清除所有 Mock
     */
    clearMocks(): void;

    /**
     * 快照测试
     */
    assertSnapshot(name: string, value: unknown, update?: boolean): Promise<void>;

    /**
     * 获取所有测试结果
     */
    getResults(): TestSuiteResult;

    /**
     * 清空测试结果
     */
    clear(): void;

    /**
     * 生成 HTML 格式的测试报告
     */
    toHTML(): string;

    /**
     * 生成 JSON 格式的测试报告
     */
    toJSON(): string;
  }

  /**
   * Page 函数类型
   */
  type PageFunction<T> = (ctx: PageContext, deps: AppDeps) => Promise<T> | T;

  /**
   * Page 包装器类型
   */
  type Page<T> = (fn: PageFunction<T>) => (ctx: PageContext) => Promise<T>;

  /**
   * MySQL 客户端接口
   * 提供完整的数据库操作功能
   */
  interface MySQLClient {
    // ========== Schema-first API ==========

    /**
     * 多行查询 - 使用 Zod schema 验证每一行
     * @param schema - Zod schema 用于验证和推断返回类型
     * @param sql - SQL 语句
     * @param params - 查询参数
     * @returns 验证后的数据数组
     */
    query<T extends ZodTypeAny>(
      schema: T,
      sql: string,
      params?: unknown[]
    ): Promise<ZodInfer<T>[]>;

    /**
     * 严格单行查询 - 必须返回且仅返回一行
     * @param schema - Zod schema 用于验证和推断返回类型
     * @param sql - SQL 语句
     * @param params - 查询参数
     * @returns 验证后的单行数据
     * @throws Error 当返回 0 行或多于 1 行时
     */
    queryOne<T extends ZodTypeAny>(
      schema: T,
      sql: string,
      params?: unknown[]
    ): Promise<ZodInfer<T>>;

    /**
     * 可选单行查询 - 返回 0 或 1 行
     * @param schema - Zod schema 用于验证和推断返回类型
     * @param sql - SQL 语句
     * @param params - 查询参数
     * @returns 验证后的单行数据，如果无结果则返回 null
     * @throws Error 当返回多于 1 行时
     */
    queryMaybe<T extends ZodTypeAny>(
      schema: T,
      sql: string,
      params?: unknown[]
    ): Promise<ZodInfer<T> | null>;

    /**
     * 单值查询 - SQL 必须使用 `AS value` 别名
     * @param schema - Zod schema 用于验证和推断返回类型
     * @param sql - SQL 语句（必须包含 `AS value` 别名）
     * @param params - 查询参数
     * @returns 验证后的单个值
     * @throws Error 当结果中没有 `value` 字段时
     */
    scalar<T extends ZodTypeAny>(
      schema: T,
      sql: string,
      params?: unknown[]
    ): Promise<ZodInfer<T>>;

    /**
     * 写操作 - INSERT/UPDATE/DELETE
     * @param schema - Zod schema 用于验证返回结果（必须包含 affectedRows 和 insertId）
     * @param sql - SQL 语句
     * @param params - 查询参数
     * @returns 验证后的执行结果（包含 affectedRows 和 insertId）
     */
    execute<T extends ZodObject<{
      affectedRows: ZodNumber;
      insertId: ZodNumber;
    }>>(
      schema: T,
      sql: string,
      params?: unknown[]
    ): Promise<ZodInfer<T>>;

    /**
     * 事务操作 - 自动提交/回滚
     * @param callback - 事务回调函数，接收事务客户端
     * @returns 回调函数的返回值
     * @throws Error 当回调抛出错误时，事务自动回滚并重新抛出错误
     */
    tx<T>(callback: (tx: MySQLClient) => Promise<T>): Promise<T>;

    /**
     * 分页查询 - 返回标准分页结构
     * @param rowSchema - Zod schema 用于验证每一行数据
     * @param sql - SQL 语句（必须包含 `AS value` 别名用于计算总数）
     * @param params - 查询参数
     * @param pageArgs - 分页参数（page 和 pageSize）
     * @returns 分页结果（items、total、page、pageSize、totalPages）
     */
    queryPage<T extends ZodTypeAny>(
      rowSchema: T,
      sql: string,
      params?: unknown[],
      pageArgs?: { page?: number; pageSize?: number }
    ): Promise<{
      items: ZodInfer<T>[];
      total: number;
      page: number;
      pageSize: number;
      totalPages: number;
    }>;

    /**
     * 关闭连接
     */
    close(): Promise<void>;
  }

  /**
   * MySQL 连接配置
   */
  interface MySQLConfig {
    /** MySQL 主机地址 */
    host: string;
    /** MySQL 端口 */
    port?: number;
    /** 数据库名称 */
    user: string;
    /** 密码 */
    password: string;
    /** 数据库名称 */
    database: string;
    /** 字符集 */
    charset?: string;
    /** 连接池配置 */
    pool?: {
      /** 最大连接数 */
      max?: number;
      /** 最小连接数 */
      min?: number;
    };
  }

  /**
   * MySQL 工厂函数类型
   * 用于创建 MySQL 客户端实例
   * @param config - 数据库连接配置
   * @param zod - Zod 实例（从依赖注入获取）
   * @returns MySQL 客户端实例
   */
  type MySQLFactory = (
    config: MySQLConfig,
    zod: unknown
  ) => Promise<MySQLClient>;

  /**
   * Redis 客户端接口
   * 提供完整的 Redis 操作功能
   */
  interface RedisClient {
    /**
     * 设置键值对
     * @param key - 键
     * @param value - 值
     * @param ttl - 过期时间（秒），可选
     */
    set(key: string, value: string, ttl?: number): Promise<void>;

    /**
     * 获取键值
     * @param key - 键
     * @returns 值，如果不存在则返回 null
     */
    get(key: string): Promise<string | null>;

    /**
     * 删除键
     * @param keys - 一个或多个键
     * @returns 删除的键数量
     */
    del(...keys: string[]): Promise<number>;

    /**
     * 检查键是否存在
     * @param key - 键
     * @returns 是否存在
     */
    exists(key: string): Promise<boolean>;

    /**
     * 设置键的过期时间
     * @param key - 键
     * @param seconds - 过期时间（秒）
     * @returns 是否设置成功
     */
    expire(key: string, seconds: number): Promise<boolean>;

    /**
     * 获取键的剩余生存时间
     * @param key - 键
     * @returns 剩余秒数，-1 表示永不过期，-2 表示键不存在
     */
    ttl(key: string): Promise<number>;

    /**
     * 列表操作：在列表左侧推入元素
     */
    lpush(key: string, ...values: string[]): Promise<number>;

    /**
     * 列表操作：在列表右侧推入元素
     */
    rpush(key: string, ...values: string[]): Promise<number>;

    /**
     * 列表操作：从列表左侧弹出元素
     */
    lpop(key: string): Promise<string | null>;

    /**
     * 列表操作：从列表右侧弹出元素
     */
    rpop(key: string): Promise<string | null>;

    /**
     * 列表操作：获取列表指定范围内的元素
     */
    lrange(key: string, start: number, stop: number): Promise<string[]>;

    /**
     * 集合操作：向集合添加成员
     */
    sadd(key: string, ...members: string[]): Promise<number>;

    /**
     * 集合操作：获取集合所有成员
     */
    smembers(key: string): Promise<string[]>;

    /**
     * 集合操作：移除并返回集合中的一个随机成员
     */
    spop(key: string): Promise<string | null>;

    /**
     * 集合操作：检查成员是否在集合中
     */
    sismember(key: string, member: string): Promise<boolean>;

    /**
     * 哈希操作：设置哈希字段值
     */
    hset(key: string, field: string, value: string): Promise<number>;

    /**
     * 哈希操作：获取哈希字段值
     */
    hget(key: string, field: string): Promise<string | null>;

    /**
     * 哈希操作：获取哈希所有字段和值
     */
    hgetall(key: string): Promise<Record<string, string>>;

    /**
     * 哈希操作：删除哈希字段
     */
    hdel(key: string, ...fields: string[]): Promise<number>;

    /**
     * 有序集合操作：添加或更新成员
     */
    zadd(key: string, score: number, member: string): Promise<number>;

    /**
     * 有序集合操作：按分数范围返回成员
     */
    zrange(key: string, start: number, stop: number): Promise<string[]>;

    /**
     * 有序集合操作：移除成员
     */
    zrem(key: string, ...members: string[]): Promise<number>;

    /**
     * 递增操作
     */
    incr(key: string): Promise<number>;

    /**
     * 递减操作
     */
    decr(key: string): Promise<number>;

    /**
     * 按指定值递增
     */
    incrBy(key: string, increment: number): Promise<number>;

    /**
     * 发布消息到频道
     */
    publish(channel: string, message: string): Promise<number>;

    /**
     * 订阅频道
     * @param channel - 频道名
     * @param callback - 消息回调函数
     */
    subscribe(
      channel: string,
      callback: (message: string) => void | Promise<void>
    ): Promise<void>;

    /**
     * 关闭连接
     */
    close(): Promise<void>;
  }

  /**
   * Redis 连接配置
   */
  interface RedisConfig {
    /** Redis 主机地址 */
    host: string;
    /** Redis 端口 */
    port?: number;
    /** 密码 */
    password?: string;
    /** 数据库索引（0-15） */
    database?: number;
  }

  /**
   * Redis 工厂函数类型
   * 用于创建 Redis 客户端实例
   */
  type RedisFactory = (config: RedisConfig) => Promise<RedisClient>;

  /**
   * LDAP 客户端接口
   * 提供完整的 LDAP 操作功能
   */
  interface LdapClient {
    /**
     * 绑定到 LDAP 服务器（认证）
     * @param dn - 可分辨名称（Distinguished Name）
     * @param password - 密码
     */
    bind(dn: string, password: string): Promise<void>;

    /**
     * 匿名绑定
     */
    anonymousBind(): Promise<void>;

    /**
     * 搜索 LDAP 目录
     * @param baseDN - 搜索基准 DN
     * @param options - 搜索选项
     * @returns 搜索结果（Entry 对象数组）
     */
    search(
      baseDN: string,
      options?: {
        /** 搜索范围：base（仅基准）、one（一级）、sub（子树，默认） */
        scope?: "base" | "one" | "sub";
        /** 搜索过滤器 */
        filter?: string;
        /** 要返回的属性列表，null 返回所有属性 */
        attributes?: string[] | null;
        /** 返回条目数量限制 */
        sizeLimit?: number;
        /** 超时时间（秒） */
        timeout?: number;
      }
    ): Promise<LdapEntry[]>;

    /**
     * 添加条目
     * @param dn - 条目的 DN
     * @param entry - 条目属性
     */
    add(dn: string, entry: Record<string, string | string[]>): Promise<void>;

    /**
     * 修改条目
     * @param dn - 条目的 DN
     * @param changes - 修改操作数组
     */
    modify(
      dn: string,
      changes: Array<{
        operation: "add" | "delete" | "replace";
        modification: Record<string, string | string[]>;
      }>
    ): Promise<void>;

    /**
     * 删除条目
     * @param dn - 要删除的条目 DN
     */
    del(dn: string): Promise<void>;

    /**
     * 修改条目的 DN（重命名或移动）
     * @param dn - 当前 DN
     * @param newDN - 新 DN
     * @param oldRDN - 是否删除旧的 RDN 属性值
     */
    modifyDN(dn: string, newDN: string, oldRDN?: boolean): Promise<void>;

    /**
     * 比较属性值
     * @param dn - 条目 DN
     * @param attribute - 属性名
     * @param value - 要比较的值
     * @returns 是否匹配
     */
    compare(dn: string, attribute: string, value: string): Promise<boolean>;

    /**
     * 关闭连接
     */
    close(): Promise<void>;

    /**
     * 检查连接是否已绑定
     */
    isBound(): boolean;
  }

  /**
   * LDAP 条目接口
   * 表示 LDAP 搜索返回的条目
   */
  interface LdapEntry {
    /** 条目的 DN */
    dn: string;
    /** 条目属性集合 */
    attributes: Record<string, string[]>;
  }

  /**
   * LDAP 连接配置
   */
  interface LdapConfig {
    /** LDAP 服务器地址 */
    url: string;
    /** 绑定 DN（用于管理员绑定） */
    bindDN?: string;
    /** 绑定密码（用于管理员绑定） */
    bindCredentials?: string;
    /** 是否使用 TLS（StartTLS） */
    startTLS?: boolean;
    /** 连接超时时间（毫秒） */
    timeout?: number;
    /** 基准 DN（用于搜索操作） */
    baseDN?: string;
    /** 是否启用详细日志 */
    verbose?: boolean;
  }

  /**
   * LDAP 工厂函数类型
   * 用于创建 LDAP 客户端实例
   */
  type LdapFactory = (config: LdapConfig) => Promise<LdapClient>;

  // ============ ExcelJS 工厂函数类型 ============

  /**
   * ExcelJS 工厂函数类型
   * 返回 ExcelJS 库实例，可直接使用其完整 API
   */
  type ExcelJSFactory = () => Promise<any>;

  /**
   * 应用依赖类型
   * 在此声明所有可注入的依赖及其类型
   *
   * @example
   * ```typescript
   * // 添加新的依赖类型
   * interface AppDeps {
   *   testFunc: () => string;
   *   createMySQL: MySQLFactory;
   *   createRedis: RedisFactory;
   * }
   * ```
   */
  interface AppDeps extends Record<string, unknown> {
    /**
     * 测试函数
     */
    testFunc: () => string;

    /**
     * MySQL 客户端工厂函数
     * 在 TSX 中调用以创建数据库连接
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createMySQL, createZod, response }) {
     *   const z = await createZod();
     *   const db = await createMySQL({
     *     host: '127.0.0.1',
     *     port: 3306,
     *     user: 'test_user',
     *     password: 'test123456',
     *     database: 'test_db'
     *   }, z);
     *
     *   const UserSchema = z.object({ id: z.number(), name: z.string() });
     *   const users = await db.query(UserSchema, 'SELECT * FROM users');
     *   return response.json(users);
     * });
     * ```
     */
    createMySQL: MySQLFactory;

    /**
     * Redis 客户端工厂函数
     * 在 TSX 中调用以创建 Redis 连接
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createRedis, response }) {
     *   const redis = await createRedis({
     *     host: '127.0.0.1',
     *     port: 6379,
     *     password: 'your_password',
     *     database: 0
     *   });
     *
     *   await redis.set('key', 'value');
     *   const value = await redis.get('key');
     *   return response.json({ key, value });
     * });
     * ```
     */
    createRedis: RedisFactory;

    /**
     * LDAP 客户端工厂函数
     * 在 TSX 中调用以创建 LDAP 连接
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createLdap, response }) {
     *   const ldap = await createLdap({
     *     url: 'ldap://127.0.0.1:389',
     *     bindDN: 'cn=admin,dc=example,dc=org',
     *     bindCredentials: 'password',
     *     baseDN: 'dc=example,dc=org'
     *   });
     *
     *   // 搜索用户
     *   const entries = await ldap.search('dc=example,dc=org', {
     *     filter: '(objectClass=person)',
     *     scope: 'sub'
     *   });
     *
     *   return response.json(entries);
     * });
     * ```
     */
    createLdap: LdapFactory;

    /**
     * ExcelJS 工厂函数
     * 返回 ExcelJS 库实例，可直接使用其完整 API
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createExcelJS, response }) {
     *   const ExcelJS = await createExcelJS();
     *
     *   // 读取 Excel 文件
     *   const workbook = new ExcelJS.Workbook();
     *   await workbook.xlsx.readFile('./data.xlsx');
     *
     *   // 获取工作表数据
     *   const worksheet = workbook.getWorksheet('Sheet1');
     *   const data: any[] = [];
     *   worksheet?.eachRow((row) => {
     *     data.push(row.values);
     *   });
     *
     *   // 写入 Excel 文件
     *   const newWorkbook = new ExcelJS.Workbook();
     *   const newSheet = newWorkbook.addWorksheet('Users');
     *   newSheet.addRow(['Name', 'Age']);
     *   newSheet.addRow(['Alice', 25]);
     *   await newWorkbook.xlsx.writeFile('./output.xlsx');
     *
     *   return response.json({ data });
     * });
     * ```
     */
    createExcelJS: ExcelJSFactory;

    /**
     * Crypto 加密工具
     * 基于 Deno 原生 Web Crypto API，提供常用的加密功能
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { crypto, response }) {
     *   // 生成随机数
     *   const iv = crypto.getRandomValues(12);
     *
     *   // SHA-256 哈希
     *   const hash = await crypto.digest('SHA-256', 'Hello World');
     *
     *   // 生成 AES-GCM 密钥
     *   const key = await crypto.generateKey('AES-GCM', 256);
     *
     *   // 加密数据
     *   const data = new TextEncoder().encode('secret message');
     *   const encrypted = await crypto.encrypt(data, key, iv);
     *
     *   // 解密数据
     *   const decrypted = await crypto.decrypt(encrypted, key, iv);
     *
     *   return response.json({
     *     hash: Array.from(new Uint8Array(hash)),
     *     decrypted: new TextDecoder().decode(decrypted)
     *   });
     * });
     * ```
     */
    crypto: {
      /** 生成随机数 */
      getRandomValues(length: number): Uint8Array;
      /** 计算哈希 (SHA-1, SHA-256, SHA-384, SHA-512, MD5 等) */
      digest(algo: string, data: string | Uint8Array): Promise<ArrayBuffer>;
      /** 生成加密密钥 (AES-GCM 或 HMAC) */
      generateKey(
        algo: "AES-GCM" | "HMAC",
        length?: number,
        extractable?: boolean,
        usages?: ("encrypt" | "decrypt" | "sign" | "verify")[],
      ): Promise<CryptoKey>;
      /** 导入密钥 */
      importKey(
        algo: "AES-GCM" | "HMAC",
        keyData: string | Uint8Array,
        extractable?: boolean,
        usages?: ("encrypt" | "decrypt" | "sign" | "verify")[],
      ): Promise<CryptoKey>;
      /** AES-GCM 加密 */
      encrypt(data: Uint8Array, key: CryptoKey, iv: Uint8Array): Promise<ArrayBuffer>;
      /** AES-GCM 解密 */
      decrypt(data: Uint8Array, key: CryptoKey, iv: Uint8Array): Promise<ArrayBuffer>;
      /** HMAC 签名 */
      sign(algo: string, key: CryptoKey, data: Uint8Array): Promise<ArrayBuffer>;
      /** HMAC 验证 */
      verify(algo: string, key: CryptoKey, signature: Uint8Array, data: Uint8Array): Promise<boolean>;
    };

    /**
     * Bcryptjs 密码哈希工具
     * 用于密码的安全哈希和验证
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createBcryptjs, response }) {
     *   const bcrypt = await createBcryptjs({ saltRounds: 10 });
     *
     *   // 哈希密码
     *   const hash = bcrypt.hash('myPassword');
     *
     *   // 验证密码
     *   const valid = bcrypt.compare('myPassword', hash);
     *
     *   return response.json({ hash, valid });
     * });
     * ```
     */
    createBcryptjs: (config?: { saltRounds?: number }) => Promise<{
      /** 哈希密码 */
      hash(password: string): string;
      /** 验证密码 */
      compare(password: string, hash: string): boolean;
    }>;

    /**
     * Session 管理
     * 通用的键值存储，用于管理会话数据
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { session }) {
     *   // 初始化 session（如果不存在）
     *   await session.init();
     *
     *   // 存储数据
     *   await session.set('userId', '123');
     *   await session.set('userName', 'Alice');
     *
     *   // 读取数据
     *   const userId = await session.get('userId');
     *   const userName = await session.get('userName');
     *
     *   // 获取所有数据
     *   const allData = await session.all();
     *
     *   // 删除数据
     *   await session.delete('userName');
     *
     *   // 清空所有数据
     *   await session.clear();
     *
     *   // 销毁 session
     *   await session.destroy();
     *
     *   return <div>Hello, {userName}</div>;
     * });
     * ```
     */
    session: {
      /** 创建新 session（如果不存在）或返回现有 session */
      init(): Promise<void>;
      /** 销毁当前 session */
      destroy(): Promise<void>;
      /** Set session data */
      set(key: string, value: unknown): Promise<void>;
      /** Get session data */
      get<T = unknown>(key: string): Promise<T | null>;
      /** Delete session data */
      delete(key: string): Promise<void>;
      /** 清空所有 session 数据 */
      clear(): Promise<void>;
      /** Regenerate session ID (prevents session fixation attacks) */
      regenerateId(): Promise<void>;
      /** Refresh session expiration time */
      touch(): Promise<void>;
      /** Check if session is valid */
      isValid(): Promise<boolean>;
      /** Get current session ID */
      getId(): string;
      /** Get all session data */
      all(): Promise<Record<string, unknown>>;
    };

    /**
     * Cookie管理器
     */
    cookies: {
      /** Set a single cookie */
      set(name: string, value: string, options?: {
        expires?: string | Date;
        maxAge?: number;
        domain?: string;
        path?: string;
        secure?: boolean;
        httpOnly?: boolean;
        sameSite?: "Strict" | "Lax" | "None";
      }): void;
      /** Delete a single cookie */
      delete(name: string, options?: {
        domain?: string;
        path?: string;
      }): void;
      /** Set multiple cookies at once */
      setMultiple(cookies: {
        [key: string]: {
          value: string;
          options?: {
            expires?: string | Date;
            maxAge?: number;
            domain?: string;
            path?: string;
            secure?: boolean;
            httpOnly?: boolean;
            sameSite?: "Strict" | "Lax" | "None";
          };
        };
      }): void;
      /** Delete multiple cookies at once */
      deleteMultiple(names: string[], options?: {
        domain?: string;
        path?: string;
      }): void;
    };

    /**
     * Response 辅助器
     * 提供便捷的 HTTP 响应创建方法
     */
    response: ResponseHelper;

    /**
     * 日志记录器
     * 提供结构化的日志记录功能
     */
    logger: {
      /** 调试日志 */
      debug(...args: unknown[]): void;
      /** 信息日志 */
      info(...args: unknown[]): void;
      /** 警告日志 */
      warn(...args: unknown[]): void;
      /** 错误日志 */
      error(...args: unknown[]): void;
    };

    /**
     * Zod 验证库工厂函数
     * 返回 Zod 库实例，提供完整的类型安全的验证功能
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createZod, body }) {
     *   const z = await createZod();
     *   const schema = z.object({
     *     name: z.string().min(2),
     *     email: z.string().email()
     *   });
     *   const userData = body(schema);
     *   return <div>Hello, {userData.name}</div>;
     * });
     * ```
     */
    createZod: () => Promise<any>;

    /**
     * Nanoid 工厂函数
     * 返回 nanoid 函数，用于生成 URL-safe 的唯一标识符
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createNanoid }) {
     *   const nanoid = await createNanoid();
     *   const id = nanoid(); // "V1StGXR8_Z5jdHi6B-myT"
     *   const customId = nanoid(10); // "V1StGXR8_Z5"
     *   return <div>ID: {id}</div>;
     * });
     * ```
     */
    createNanoid: () => Promise<(size?: number) => string>;

    /**
     * TSP Info - 服务器信息查看器
     * 类似 PHP 的 phpinfo()，显示服务器运行时信息
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { tspinfo }) {
     *   // 返回 HTML 格式的服务器信息页面
     *   return tspinfo.renderHTML();
     * });
     * ```
     */
    tspinfo: {
      /** 渲染为 HTML 格式的服务器信息页面 */
      renderHTML(): Promise<string>;
      /** 获取服务器信息对象 */
      getData(): Promise<{
        server: {
          tspVersion: string;
          architecture: string;
          target: string;
        };
        config: Record<string, unknown>;
        runtime: Record<string, unknown>;
        system: Record<string, unknown>;
        dependencies: string[];
      }>;
    };

    /**
     * 测试辅助器 - 内置测试框架
     * 类似 Jest/PHPUnit，无需额外安装工具
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { testHelper, response }) {
     *   // 清空之前的测试结果
     *   testHelper.clear();
     *
     *   // 运行测试
     *   await testHelper.test('1 + 1 = 2', () => {
     *     testHelper.assertEqual(1 + 1, 2);
     *   });
     *
     *   await testHelper.test('API 返回用户列表', async () => {
     *     const resp = await fetch('/api/users');
     *     const users = await resp.json();
     *     testHelper.assertTrue(Array.isArray(users));
     *   });
     *
     *   // 获取测试结果
     *   const results = testHelper.getResults();
     *
     *   // 返回 JSON 格式结果
     *   return response.json(results);
     * });
     * ```
     */
    testHelper: {
      /** 运行单个测试 */
      test(name: string, fn: () => void | Promise<void>): Promise<{
        name: string;
        passed: boolean;
        error?: string;
        data?: unknown;
        duration?: number;
      }>;
      /** 断言相等 */
      assertEqual(actual: unknown, expected: unknown, message?: string): void;
      /** 断言不相等 */
      assertNotEqual(actual: unknown, expected: unknown, message?: string): void;
      /** 断言为真 */
      assertTrue(value: unknown, message?: string): void;
      /** 断言为假 */
      assertFalse(value: unknown, message?: string): void;
      /** 断言为 null */
      assertNull(value: unknown, message?: string): void;
      /** 断言不为 null */
      assertNotNull(value: unknown, message?: string): void;
      /** 断言包含 */
      assertContains(haystack: string | unknown[], needle: unknown, message?: string): void;
      /** 断言抛出异常 */
      assertThrows(fn: () => void, expectedError?: string | RegExp, message?: string): void;

      /** 直接让测试失败 */
      fail(message?: string): void;

      /**
       * ⭐ 新增：模拟 PageContext
       * 创建一个模拟的页面上下文，用于直接测试页面函数
       *
       * @example
       * ```tsx
       * const mockCtx = testHelper.mockContext({
       *   method: 'POST',
       *   url: 'http://localhost:9000/api/user',
       *   query: { id: '123' },
       *   body: { name: 'Alice', age: 25 }
       * });
       * ```
       */
      mockContext: (ctx: {
        method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
        url: string | URL;
        headers?: Record<string, string>;
        query?: Record<string, string>;
        body?: unknown;
        cookies?: Record<string, string>;
      }) => PageContext;

      /**
       * ⭐ 新增：直接调用页面函数（单元测试）
       * 不经过 HTTP 层，直接调用并测试页面函数
       *
       * @example
       * ```tsx
       * import userPage from './user.tsx';
       *
       * const mockCtx = testHelper.mockContext({
       *   method: 'GET',
       *   url: 'http://localhost:9000/user/123'
       * });
       *
       * const result = await testHelper.runPage(userPage, mockCtx);
       * testHelper.assertEqual(result.status, 200);
       * ```
       */
      runPage: (
        pageFn: (ctx: PageContext, deps: any) => Promise<unknown>,
        ctx: PageContext
      ) => Promise<unknown>;

      /**
       * ⭐ 新增：Mock HTTP 请求
       * 拦截并模拟 fetch 请求，避免真实网络调用
       *
       * @param urlPattern - URL 匹配模式（字符串或正则表达式）
       * @param options - 模拟选项
       *
       * @example
       * ```tsx
       * // Mock 特定 URL
       * testHelper.mockFetch('/api/users', {
       *   response: [{ id: 1, name: 'Alice' }],
       *   status: 200
       * });
       *
       * // Mock 正则匹配
       * testHelper.mockFetch(/\/api\/users\/\d+/, {
       *   response: { id: 1, name: 'Alice' }
       * });
       * ```
       */
      mockFetch: (
        urlPattern: string | RegExp,
        options: {
          response: unknown;
          status?: number;
          headers?: Record<string, string>;
          delay?: number;
        }
      ) => void;

      /**
       * ⭐ 新增：清除所有 Mock
       * 清除所有 fetch mock，恢复真实网络调用
       */
      clearMocks: () => void;

      /**
       * ⭐ 新增：快照测试
       * 将值与快照文件比较，用于 UI 回归测试
       *
       * @param name - 快照名称
       * @param value - 要快照的值
       * @param update - 是否更新快照（默认 false）
       *
       * @example
       * ```tsx
       * testHelper.assertSnapshot('user-list', users);
       * ```
       */
      assertSnapshot: (name: string, value: unknown, update?: boolean) => Promise<void>;

      /** 获取所有测试结果 */
      getResults(): {
        total: number;
        passed: number;
        failed: number;
        results: Array<{
          name: string;
          passed: boolean;
          error?: string;
          data?: unknown;
          duration?: number;
        }>;
        duration: number;
      };
      /** 清空测试结果 */
      clear(): void;
      /** 生成 HTML 格式的测试报告 */
      toHTML(): string;
      /** 生成 JSON 格式的测试报告 */
      toJSON(): string;
    };

    /**
     * 类型定义集合
     * 提供所有业务类型定义（PageContext、RedirectResult 等）
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { types }) {
     *   // 使用类型进行类型检查或类型断言
     *   const context: PageContext = ctx;
     *   // 或访问类型名称
     *   const typeName = types.names.PageContext;
     *   return <div>Hello</div>;
     * });
     * ```
     */
    types: {
      /** 类型名称映射（用于调试和反射） */
      readonly names: {
        readonly HttpMethod: "HttpMethod";
        readonly PageContext: "PageContext";
        readonly RedirectResult: "RedirectResult";
        readonly UploadedFile: "UploadedFile";
        readonly ResponseHelper: "ResponseHelper";
        readonly Logger: "Logger";
        readonly LogRotationConfig: "LogRotationConfig";
        readonly LoggerConfig: "LoggerConfig";
        readonly FileManagerConfig: "FileManagerConfig";
        readonly MySQLClient: "MySQLClient";
        readonly MySQLConfig: "MySQLConfig";
        readonly MySQLFactory: "MySQLFactory";
        readonly RedisClient: "RedisClient";
        readonly RedisConfig: "RedisConfig";
        readonly RedisFactory: "RedisFactory";
        readonly LdapClient: "LdapClient";
        readonly LdapConfig: "LdapConfig";
        readonly LdapEntry: "LdapEntry";
        readonly ExcelJSFactory: "ExcelJSFactory";
        readonly TestResult: "TestResult";
        readonly TestSuiteResult: "TestSuiteResult";
        readonly MockPageContext: "MockPageContext";
        readonly MockResponse: "MockResponse";
        readonly MockFetchOptions: "MockFetchOptions";
        readonly TestHelper: "TestHelper";
        readonly PageFunction: "PageFunction";
        readonly Page: "Page";
        readonly AppDeps: "AppDeps";
      };
      /** 类型构造器（占位符，提供类型信息） */
      readonly types: {
        HttpMethod: HttpMethod;
        PageContext: PageContext;
        RedirectResult: RedirectResult;
        UploadedFile: UploadedFile;
        ResponseHelper: ResponseHelper;
        Logger: Logger;
        LogRotationConfig: LogRotationConfig;
        LoggerConfig: LoggerConfig;
        FileManagerConfig: FileManagerConfig;
        MySQLClient: MySQLClient;
        MySQLConfig: MySQLConfig;
        MySQLFactory: MySQLFactory;
        RedisClient: RedisClient;
        RedisConfig: RedisConfig;
        RedisFactory: RedisFactory;
        LdapClient: LdapClient;
        LdapConfig: LdapConfig;
        LdapEntry: LdapEntry;
        ExcelJSFactory: ExcelJSFactory;
        TestResult: TestResult;
        TestSuiteResult: TestSuiteResult;
        MockPageContext: MockPageContext;
        MockResponse: MockResponse;
        MockFetchOptions: MockFetchOptions;
        TestHelper: TestHelper;
        PageFunction: PageFunction<unknown>;
        Page: Page<unknown>;
        AppDeps: AppDeps;
      };
    };

    /**
     * Zod 验证库
     * 提供完整的类型安全的验证功能
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createZod, body }) {
     *   const z = await createZod();
     *   const schema = z.object({
     *     name: z.string().min(2),
     *     email: z.string().email()
     *   });
     *   const userData = body(schema);
     *   return <div>Hello, {userData.name}</div>;
     * });
     * ```
     */

    /**
     * 请求体验证函数
     * 从 PageContext.body 解析并验证数据
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createZod, body, response }) {
     *   const z = await createZod();
     *   const userSchema = z.object({
     *     name: z.string().min(2),
     *     email: z.string().email(),
     *     age: z.coerce.number().min(1).max(150).optional()
     *   });
     *   const userData = body(userSchema);
     *   return response.json({ success: true, data: userData });
     * });
     * ```
     */
    body: <T>(schema: any) => T;

    /**
     * 查询参数验证函数
     * 从 PageContext.query 解析并验证数据
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createZod, query }) {
     *   const z = await createZod();
     *   const { page, limit, keyword } = query(z.object({
     *     page: z.coerce.number().min(1).default(1),
     *     limit: z.coerce.number().min(1).max(100).default(10),
     *     keyword: z.string().optional()
     *   }));
     *   return <div>第 {page} 页，每页 {limit} 条，搜索: {keyword || '全部'}</div>;
     * });
     * ```
     */
    query: <T>(schema: any) => T;

    /**
     * 格式化 Zod 错误
     * 将 ZodError 转换为友好的错误信息
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { createZod, body, formatZodError, response }) {
     *   const z = await createZod();
     *   try {
     *     const userData = body(z.object({
     *       name: z.string().min(2),
     *       email: z.string().email()
     *     }));
     *     return response.json({ success: true, data: userData });
     *   } catch (error) {
     *     if (error instanceof z.ZodError) {
     *       const formatted = formatZodError(error);
     *       return response.json(formatted, 400);
     *     }
     *     throw error;
     *   }
     * });
     * ```
     */
    formatZodError: (error: any) => {
      success: boolean;
      message: string;
      errors: Array<{
        path: string[];
        message: string;
        code: string;
      }>;
    };
  }

  /**
   * 全局 Page 函数
   * TSX 文件中可以直接使用，无需 import
   *
   * @example
   * ```tsx
   * export default Page(async function(ctx, { testFunc, db }) {
   *   const result = testFunc();  // ✅ 有完整类型提示
   *   return <div>{result}</div>;
   * });
   * ```
   */
  function Page<T>(
    fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
  ): (ctx: PageContext) => Promise<T>;
}

// 确保类型被视为全局的
export {};
