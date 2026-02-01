/**
 * LDAP 客户端实现类
 * 提供完整的 LDAP 操作功能，基于 ldapts 库
 */

import { Client, type SearchOptions } from "ldapts";

/**
 * LDAP 客户端实现类
 */
export class LdapClientImpl implements globalThis.LdapClient {
  private client: Client;
  private bound: boolean = false;
  private config: globalThis.LdapConfig;

  constructor(config: globalThis.LdapConfig) {
    this.config = config;

    this.client = new Client({
      url: config.url,
      timeout: config.timeout || 10000,
    });
  }

  /**
   * 绑定到 LDAP 服务器（认证）
   */
  async bind(dn: string, password: string): Promise<void> {
    await this.client.bind(dn, password);
    this.bound = true;
  }

  /**
   * 匿名绑定
   */
  async anonymousBind(): Promise<void> {
    await this.client.bind("", "");
    this.bound = true;
  }

  /**
   * 搜索 LDAP 目录
   */
  async search(
    baseDN: string,
    options?: {
      scope?: "base" | "one" | "sub";
      filter?: string;
      attributes?: string[] | null;
      sizeLimit?: number;
      timeout?: number;
    }
  ): Promise<globalThis.LdapEntry[]> {
    const searchOptions: SearchOptions = {
      scope: options?.scope || "sub",
      filter: options?.filter || "(objectClass=*)",
      attributes: options?.attributes || undefined,
      sizeLimit: options?.sizeLimit,
      paged: true, // 启用分页以处理大结果集
    };

    const { searchEntries } = await this.client.search(baseDN, searchOptions);

    // 转换为标准格式
    return searchEntries.map((entry) => ({
      dn: entry.dn,
      attributes: this.convertAttributes(
        entry.attributes as unknown as Record<string, unknown>
      ),
    }));
  }

  /**
   * 转换 ldapts 属性格式为标准格式
   */
  private convertAttributes(
    attrs: Record<string, unknown>
  ): Record<string, string[]> {
    const result: Record<string, string[]> = {};

    for (const [key, value] of Object.entries(attrs)) {
      if (Array.isArray(value)) {
        // 处理数组
        result[key] = value.map((v) => String(v));
      } else {
        // 处理单个值
        result[key] = [String(value)];
      }
    }

    return result;
  }

  /**
   * 添加条目
   */
  async add(dn: string, entry: Record<string, string | string[]>): Promise<void> {
    await this.client.add(dn, entry);
  }

  /**
   * 修改条目
   */
  async modify(
    dn: string,
    changes: Array<{
      operation: "add" | "delete" | "replace";
      modification: Record<string, string | string[]>;
    }>
  ): Promise<void> {
    // 转换为 ldapts 期望的格式
    for (const change of changes) {
      const modification = {
        operation: change.operation,
        modification: change.modification,
      };
      await this.client.modify(dn, modification as never);
    }
  }

  /**
   * 删除条目
   */
  async del(dn: string): Promise<void> {
    // ldapts 使用 del 方法
    await (this.client as unknown as { del(dn: string): Promise<void> }).del(dn);
  }

  /**
   * 修改条目的 DN（重命名或移动）
   */
  async modifyDN(dn: string, newDN: string, _oldRDN?: boolean): Promise<void> {
    await this.client.modifyDN(dn, newDN);
  }

  /**
   * 比较属性值
   */
  async compare(dn: string, attribute: string, value: string): Promise<boolean> {
    return await this.client.compare(dn, attribute, value);
  }

  /**
   * 关闭连接
   */
  async close(): Promise<void> {
    await this.client.unbind();
    this.bound = false;
  }

  /**
   * 检查连接是否已绑定
   */
  isBound(): boolean {
    return this.bound;
  }

  /**
   * 启动 TLS（如果配置了）
   */
  async startTLS(): Promise<void> {
    if (this.config.startTLS) {
      await this.client.startTLS();
    }
  }
}

/**
 * LDAP 客户端工厂函数
 */
export async function createLdapClient(
  config: globalThis.LdapConfig
): Promise<globalThis.LdapClient> {
  const client = new LdapClientImpl(config);

  // 如果配置了管理员凭据，自动绑定
  if (config.bindDN && config.bindCredentials) {
    await client.bind(config.bindDN, config.bindCredentials);
  }

  // 如果配置了 StartTLS，启动 TLS
  if (config.startTLS) {
    await client.startTLS();
  }

  return client;
}
