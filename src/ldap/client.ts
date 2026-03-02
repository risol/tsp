/**
 * LDAP client implementation class
 * Provides basic LDAP operations based on ldapts library
 */

import { Client, Change, Attribute, type SearchOptions } from "ldapts";

/**
 * LDAP client implementation class
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
   * Bind to LDAP server (authenticate)
   */
  async bind(dn: string, password: string): Promise<void> {
    await this.client.bind(dn, password);
    this.bound = true;
  }

  /**
   * Anonymous bind
   */
  async anonymousBind(): Promise<void> {
    await this.client.bind("", "");
    this.bound = true;
  }

  /**
   * Search LDAP directory
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
      paged: true, // Enable paging for large result sets
    };

    const { searchEntries } = await this.client.search(baseDN, searchOptions);

    // Convert to standard format
    // ldapts returns entry properties at root level, not nested in attributes
    return searchEntries.map((entry) => {
      // Extract dn, rest are attributes
      const { dn, ...attributes } = entry as unknown as { dn: string; [key: string]: unknown };

      return {
        dn,
        attributes: this.convertAttributes(
          attributes as Record<string, unknown>
        ),
      };
    });
  }

  /**
   * Convert ldapts attribute format to standard format
   */
  private convertAttributes(
    attrs: Record<string, unknown>
  ): Record<string, string[]> {
    // Handle null or undefined
    if (!attrs) {
      return {};
    }

    const result: Record<string, string[]> = {};

    for (const [key, value] of Object.entries(attrs)) {
      if (Array.isArray(value)) {
        // Handle array
        result[key] = value.map((v) => String(v));
      } else {
        // Handle single value
        result[key] = [String(value)];
      }
    }

    return result;
  }

  /**
   * Add entry
   */
  async add(dn: string, entry: Record<string, string | string[]>): Promise<void> {
    await this.client.add(dn, entry);
  }

  /**
   * Modify entry
   */
  async modify(
    dn: string,
    changes: Array<{
      operation: "add" | "delete" | "replace";
      modification: Record<string, string | string[]>;
    }>
  ): Promise<void> {
    // Process each modification operation separately
    for (const change of changes) {
      // Convert modification object to ldapts Attribute object
      for (const [key, value] of Object.entries(change.modification)) {
        const attribute = new Attribute({
          type: key,
          values: Array.isArray(value) ? value : [value],
        });

        const ldapChange = new Change({
          operation: change.operation,
          modification: attribute,
        });

        await this.client.modify(dn, ldapChange);
      }
    }
  }

  /**
   * Delete entry
   */
  async del(dn: string): Promise<void> {
    // ldapts uses del method
    await (this.client as unknown as { del(dn: string): Promise<void> }).del(dn);
  }

  /**
   * Modify entry DN (rename or move)
   */
  async modifyDN(dn: string, newDN: string, _oldRDN?: boolean): Promise<void> {
    await this.client.modifyDN(dn, newDN);
  }

  /**
   * Compare attribute value
   */
  async compare(dn: string, attribute: string, value: string): Promise<boolean> {
    return await this.client.compare(dn, attribute, value);
  }

  /**
   * Close connection
   */
  async close(): Promise<void> {
    await this.client.unbind();
    this.bound = false;
  }

  /**
   * Check if connection is bound
   */
  isBound(): boolean {
    return this.bound;
  }

  /**
   * Start TLS (if configured)
   */
  async startTLS(): Promise<void> {
    if (this.config.startTLS) {
      await this.client.startTLS();
    }
  }
}

/**
 * LDAP client factory function
 */
export async function createLdapClient(
  config: globalThis.LdapConfig
): Promise<globalThis.LdapClient> {
  const client = new LdapClientImpl(config);

  // If admin credentials configured, bind automatically
  if (config.bindDN && config.bindCredentials) {
    await client.bind(config.bindDN, config.bindCredentials);
  }

  // If StartTLS configured, start TLS
  if (config.startTLS) {
    await client.startTLS();
  }

  return client;
}
