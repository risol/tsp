/**
 * LDAP 客户端单元测试
 * 测试 LDAP 客户端的基本功能
 */

import { assertEquals, assertExists } from "@std/assert";
import type { LdapClient, LdapConfig, LdapEntry } from "../../types.d.ts";

Deno.test("LDAP: 配置接口验证", () => {
  const config: LdapConfig = {
    url: "ldap://localhost:389",
    bindDN: "cn=admin,dc=example,dc=org",
    bindCredentials: "password",
    baseDN: "dc=example,dc=org",
    startTLS: false,
    timeout: 10000,
    verbose: false,
  };

  assertEquals(typeof config.url, "string");
  assertEquals(config.url, "ldap://localhost:389");
  assertEquals(config.baseDN, "dc=example,dc=org");
});

Deno.test("LDAP: LdapEntry 接口验证", () => {
  const entry: LdapEntry = {
    dn: "cn=user,dc=example,dc=org",
    attributes: {
      cn: ["user"],
      mail: ["user@example.com"],
      objectClass: ["person", "organizationalPerson"],
    },
  };

  assertEquals(typeof entry.dn, "string");
  assertEquals(typeof entry.attributes, "object");
  assertEquals(entry.attributes.cn[0], "user");
});

Deno.test("LDAP: 工厂函数类型验证", async () => {
  // 注意：这个测试只验证类型，不会实际连接 LDAP 服务器
  // 实际连接测试需要真实的 LDAP 服务器

  const { createLdapClient } = await import("../../src/ldap/client.ts");

  assertExists(createLdapClient);
  assertEquals(typeof createLdapClient, "function");
});

Deno.test("LDAP: 客户端实例化测试（无需连接）", async () => {
  const { LdapClientImpl } = await import("../../src/ldap/client.ts");

  const config: globalThis.LdapConfig = {
    url: "ldap://localhost:389",
    baseDN: "dc=example,dc=org",
  };

  // 创建客户端实例（不会自动绑定）
  const client = new LdapClientImpl(config);

  assertExists(client);
  assertEquals(client.isBound(), false);

  // 注意：不调用任何需要实际连接的方法
  // close() 也不调用，因为没有建立连接
});
