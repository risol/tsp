import mysql from "npm:mysql2@^2.3.3/promise";

/**
 * MySQL 客户端实现类
 * 提供完整的数据库操作功能，包括查询、插入、更新、删除和事务支持
 */
export class MySQLClientImpl implements globalThis.MySQLClient {
  private pool: mysql.Pool;
  private connection: mysql.PoolConnection | null = null;

  constructor(config: globalThis.MySQLConfig) {
    this.pool = mysql.createPool({
      host: config.host,
      port: config.port || 3306,
      user: config.user,
      password: config.password,
      database: config.database,
      charset: config.charset || "utf8mb4",
      connectionLimit: config.pool?.max || 10,
      waitForConnections: true,
      queueLimit: 0,
    });
  }

  /**
   * 获取连接（支持事务）
   */
  private async getConnection(): Promise<mysql.PoolConnection> {
    if (!this.connection) {
      this.connection = await this.pool.getConnection();
    }
    return this.connection;
  }

  /**
   * 释放连接回连接池
   */
  private async release(): Promise<void> {
    if (this.connection) {
      this.connection.release();
      this.connection = null;
    }
  }

  /**
   * 执行查询（支持参数化查询，防止 SQL 注入）
   */
  async query<T = unknown>(
    sql: string,
    params?: unknown[]
  ): Promise<T[]> {
    const conn = await this.getConnection();
    const [rows] = await conn.execute(sql, params);
    return rows as T[];
  }

  /**
   * 执行 SQL 语句并返回完整结果（包括 insertId、affectedRows 等）
   */
  private async execute(sql: string, params?: unknown[]): Promise<mysql.ResultSetHeader> {
    const conn = await this.getConnection();
    const [result] = await conn.execute(sql, params);
    return result as mysql.ResultSetHeader;
  }

  /**
   * 插入数据
   */
  async insert(table: string, data: Record<string, unknown>): Promise<number> {
    const keys = Object.keys(data);
    const values = Object.values(data);
    const placeholders = keys.map(() => "?").join(", ");

    const sql = `INSERT INTO ${table} (${keys.join(", ")}) VALUES (${placeholders})`;
    const result = await this.execute(sql, values);

    return result.insertId;
  }

  /**
   * 更新数据
   */
  async update(
    table: string,
    data: Record<string, unknown>,
    where: Record<string, unknown>
  ): Promise<number> {
    const setClause = Object.keys(data).map(k => `${k} = ?`).join(", ");
    const whereClause = Object.keys(where).map(k => `${k} = ?`).join(" AND ");
    const values = [...Object.values(data), ...Object.values(where)];

    const sql = `UPDATE ${table} SET ${setClause} WHERE ${whereClause}`;
    const result = await this.execute(sql, values);

    return result.affectedRows;
  }

  /**
   * 删除数据
   */
  async delete(table: string, where: Record<string, unknown>): Promise<number> {
    const whereClause = Object.keys(where).map(k => `${k} = ?`).join(" AND ");
    const values = Object.values(where);

    const sql = `DELETE FROM ${table} WHERE ${whereClause}`;
    const result = await this.execute(sql, values);

    return result.affectedRows;
  }

  /**
   * 开启事务
   */
  async beginTransaction(): Promise<void> {
    const conn = await this.getConnection();
    await conn.beginTransaction();
  }

  /**
   * 提交事务
   */
  async commit(): Promise<void> {
    if (this.connection) {
      await this.connection.commit();
      await this.release();
    }
  }

  /**
   * 回滚事务
   */
  async rollback(): Promise<void> {
    if (this.connection) {
      await this.connection.rollback();
      await this.release();
    }
  }

  /**
   * 关闭连接
   */
  async close(): Promise<void> {
    await this.release();
    await this.pool.end();
  }
}
