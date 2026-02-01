/**
 * 全局类型声明
 * TSX 文件可以直接使用这些类型，无需 import
 */

declare global {
  /**
   * HTTP 方法类型
   */
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
   */
  interface PageContext {
    /** HTTP 请求方法 */
    readonly method: HttpMethod;
    /** 完整的请求 URL 对象 */
    readonly url: URL;
    /** 请求头对象 */
    readonly headers: Headers;
    /** URL 查询参数 */
    readonly query: Record<string, string>;
    /** 请求体数据 */
    readonly body: unknown;
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
   * MySQL 客户端接口
   * 提供完整的数据库操作功能
   */
  interface MySQLClient {
    /**
     * 执行查询（支持参数化查询，防止 SQL 注入）
     * @param sql - SQL 语句
     * @param params - 查询参数
     * @returns 查询结果数组
     */
    query<T = unknown>(sql: string, params?: unknown[]): Promise<T[]>;

    /**
     * 插入数据
     * @param table - 表名
     * @param data - 数据对象
     * @returns 插入行的 ID
     */
    insert(table: string, data: Record<string, unknown>): Promise<number>;

    /**
     * 更新数据
     * @param table - 表名
     * @param data - 更新的数据
     * @param where - WHERE 条件
     * @returns 影响的行数
     */
    update(
      table: string,
      data: Record<string, unknown>,
      where: Record<string, unknown>
    ): Promise<number>;

    /**
     * 删除数据
     * @param table - 表名
     * @param where - WHERE 条件
     * @returns 影响的行数
     */
    delete(table: string, where: Record<string, unknown>): Promise<number>;

    /**
     * 开启事务
     */
    beginTransaction(): Promise<void>;

    /**
     * 提交事务
     */
    commit(): Promise<void>;

    /**
     * 回滚事务
     */
    rollback(): Promise<void>;

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
   */
  type MySQLFactory = (config: MySQLConfig) => Promise<MySQLClient>;

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
     * export default Page(async function(ctx, { createMySQL, response }) {
     *   const db = await createMySQL({
     *     host: '127.0.0.1',
     *     port: 3306,
     *     user: 'test_user',
     *     password: 'test123456',
     *     database: 'test_db'
     *   });
     *
     *   const users = await db.query('SELECT * FROM users');
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
     * Session 管理
     */
    session: import("./src/session.ts").SessionManager;

    /**
     * Cookie管理器
     */
    cookies: import("./src/cookies.ts").CookieManager;

    /**
     * Response 辅助器
     * 提供便捷的 HTTP 响应创建方法
     */
    response: ResponseHelper;

    /**
     * 日志记录器
     * 提供结构化的日志记录功能
     */
    logger: import("./src/logger.ts").Logger;

    /**
     * nanoid 唯一 ID 生成器
     * 生成 URL-safe 的唯一标识符
     *
     * @example
     * ```tsx
     * export default Page(async function(ctx, { nanoid }) {
     *   const id = nanoid(); // "V1StGXR8_Z5jdHi6B-myT"
     *   const customId = nanoid(10); // "V1StGXR8_Z5"
     *   return <div>ID: {id}</div>;
     * });
     * ```
     */
    nanoid: (size?: number) => string;

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
    tspinfo: import("./src/tspinfo.ts").TspInfo;
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
