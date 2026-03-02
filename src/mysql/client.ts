import mysql from "npm:mysql2@^2.3.3/promise";

/**
 * MySQL client implementation class
 * Provides complete database operations including Schema-first queries, insert, update, delete, and transaction support
 */
export class MySQLClientImpl implements globalThis.MySQLClient {
  private connection: mysql.Connection | null = null;
  private z: any;

  constructor(config: globalThis.MySQLConfig, zod: any) {
    this.z = zod;

    // Lazy load connection, create on first query
    this.config = {
      host: config.host,
      port: config.port || 3306,
      user: config.user,
      password: config.password,
      database: config.database,
      charset: config.charset || "utf8mb4",
    };
  }

  private config: mysql.ConnectionOptions;

  /**
   * Get connection (singleton pattern)
   */
  private async getConnection(): Promise<mysql.Connection> {
    if (!this.connection) {
      this.connection = await mysql.createConnection(this.config);
    }
    return this.connection;
  }

  // ========== Schema-first API ==========

  /**
   * Multi-row query - validate each row using Zod schema
   */
  async query<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any[]> {
    const conn = await this.getConnection();
    const [rows] = await conn.execute(sql, params);

    // Validate each row
    const validatedRows: any[] = [];
    for (const row of rows as any[]) {
      const validated = (schema as any).parse(row);
      validatedRows.push(validated);
    }

    return validatedRows;
  }

  /**
   * Strict single-row query - must return exactly one row
   */
  async queryOne<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any> {
    const rows = await this.query(schema, sql, params);

    if (rows.length === 0) {
      throw new Error(`Expected 1 row, got 0`);
    }

    if (rows.length > 1) {
      throw new Error(`Expected 1 row, got ${rows.length}`);
    }

    return rows[0];
  }

  /**
   * Optional single-row query - returns 0 or 1 row
   */
  async queryMaybe<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any | null> {
    const rows = await this.query(schema, sql, params);

    if (rows.length === 0) {
      return null;
    }

    if (rows.length > 1) {
      throw new Error(`Expected 0 or 1 row, got ${rows.length}`);
    }

    return rows[0];
  }

  /**
   * Scalar query - SQL must use `AS value` alias
   */
  async scalar<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any> {
    const conn = await this.getConnection();
    const [rows] = await conn.execute(sql, params);

    if (!Array.isArray(rows) || rows.length === 0) {
      throw new Error(`Expected 1 row, got 0`);
    }

    const row = rows[0] as any;
    if (!('value' in row)) {
      throw new Error(`Missing 'value' field in result. Use 'AS value' alias in SQL.`);
    }

    return (schema as any).parse(row.value);
  }

  /**
   * Write operation - INSERT/UPDATE/DELETE
   */
  async execute<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any> {
    const conn = await this.getConnection();
    const [result] = await conn.execute(sql, params);

    const resultSetHeader = result as mysql.ResultSetHeader;
    const validated = (schema as any).parse({
      affectedRows: resultSetHeader.affectedRows,
      insertId: resultSetHeader.insertId,
    });

    return validated;
  }

  /**
   * Transaction operation - auto commit/rollback
   */
  async tx<T>(callback: (tx: globalThis.MySQLClient) => Promise<T>): Promise<T> {
    const conn = await this.getConnection();
    await conn.beginTransaction();

    // Create transaction client (reuse current connection)
    const txClient = new TransactionalClient(this, conn);

    try {
      const result = await callback(txClient);
      await conn.commit();
      return result;
    } catch (error) {
      await conn.rollback();
      throw error;
    }
  }

  /**
   * Paginated query - returns standard pagination structure
   */
  async queryPage<T extends globalThis.ZodTypeAny>(
    rowSchema: T,
    sql: string,
    params?: unknown[],
    pageArgs: { page?: number; pageSize?: number } = {}
  ): Promise<{
    items: globalThis.ZodInfer<T>[];
    total: number;
    page: number;
    pageSize: number;
    totalPages: number;
  }> {
    const page = pageArgs.page || 1;
    const pageSize = pageArgs.pageSize || 10;
    const offset = (page - 1) * pageSize;

    // Execute query
    const conn = await this.getConnection();

    // MySQL doesn't support placeholders in LIMIT/OFFSET, need to replace with actual values
    // Detect "LIMIT ? OFFSET ?" or "LIMIT ? OFFSET ?" pattern in SQL
    let finalSql = sql;
    let finalParams = params || [];

    // Regex to match LIMIT ? OFFSET ? pattern (case insensitive and extra spaces)
    const limitOffsetPattern = /\bLIMIT\s+\?\s*(?:OFFSET\s+\?)?/i;
    const limitOffsetMatch = sql.match(limitOffsetPattern);

    if (limitOffsetMatch) {
      // Remove placeholders from LIMIT ? OFFSET ?, use values directly
      finalSql = sql.replace(/\bLIMIT\s+\?\s*(?:OFFSET\s+\?)?/i, `LIMIT ${pageSize} OFFSET ${offset}`);

      // If SQL only has these two placeholders, clear params
      const paramCount = (sql.match(/\?/g) || []).length;
      if (paramCount === 2) {
        finalParams = [];
      }
    } else {
      // If no LIMIT ? OFFSET ? pattern, try adding at end
      if (!sql.toLowerCase().includes('limit')) {
        finalSql = `${sql} LIMIT ${pageSize} OFFSET ${offset}`;
      }
    }

    const [rows] = await conn.execute(finalSql, finalParams);

    // Validate each row
    const validatedItems: globalThis.ZodInfer<T>[] = [];
    let total = 0;

    for (const row of rows as any[]) {
      // Extract total field (if exists)
      if ('total' in row && typeof row.total === 'number') {
        total = row.total;
      }
      const validated = (rowSchema as any).parse(row);
      validatedItems.push(validated);
    }

    // If SQL doesn't include total field, need to query again
    if (total === 0 && validatedItems.length > 0) {
      // Try to extract COUNT(*) OVER() result from SQL
      const firstRow = (rows as any[])[0];
      if (firstRow && 'total' in firstRow) {
        total = firstRow.total;
      }
    }

    const totalPages = Math.ceil(total / pageSize);

    return {
      items: validatedItems,
      total,
      page,
      pageSize,
      totalPages,
    };
  }

  /**
   * Close connection
   */
  async close(): Promise<void> {
    if (this.connection) {
      await this.connection.end();
      this.connection = null;
    }
  }
}

/**
 * Transactional client
 * Reuses parent connection, doesn't close connection after transaction
 */
class TransactionalClient implements globalThis.MySQLClient {
  private parent: MySQLClientImpl;
  private connection: mysql.Connection;

  constructor(parent: MySQLClientImpl, connection: mysql.Connection) {
    this.parent = parent;
    this.connection = connection;
  }

  // ========== Schema-first API ==========

  query<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any[]> {
    return this.parent['query'](schema, sql, params);
  }

  queryOne<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any> {
    return this.parent['queryOne'](schema, sql, params);
  }

  queryMaybe<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any | null> {
    return this.parent['queryMaybe'](schema, sql, params);
  }

  scalar<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any> {
    return this.parent['scalar'](schema, sql, params);
  }

  execute<T extends any>(
    schema: T,
    sql: string,
    params?: unknown[]
  ): Promise<any> {
    return this.parent['execute'](schema, sql, params);
  }

  tx<T>(callback: (tx: globalThis.MySQLClient) => Promise<T>): Promise<T> {
    throw new Error('Nested transactions are not supported');
  }

  queryPage<T extends globalThis.ZodTypeAny>(
    rowSchema: T,
    sql: string,
    params?: unknown[],
    pageArgs?: { page?: number; pageSize?: number }
  ): Promise<{
    items: globalThis.ZodInfer<T>[];
    total: number;
    page: number;
    pageSize: number;
    totalPages: number;
  }> {
    return this.parent['queryPage'](rowSchema, sql, params, pageArgs);
  }

  close(): Promise<void> {
    // Transactional client doesn't close connection
    return Promise.resolve();
  }
}
