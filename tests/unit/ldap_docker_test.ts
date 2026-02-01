#!/usr/bin/env -S deno run --allow-net --allow-all

/**
 * LDAP Docker 连接测试
 * 测试与 Docker 容器中的 OpenLDAP 服务器的连接
 */

import { createLdapClient } from "../../src/ldap/client.ts";

const LDAP_CONFIG = {
  url: "ldap://localhost:1389",
  bindDN: "cn=admin,dc=example,dc=org",
  bindCredentials: "admin123456",
  baseDN: "dc=example,dc=org",
};

const TEST_USERS = [
  {
    dn: "cn=zhang san,ou=developers,dc=example,dc=org",
    password: "password123",
    name: "张三",
  },
  {
    dn: "cn=li si,ou=developers,dc=example,dc=org",
    password: "password456",
    name: "李四",
  },
  {
    dn: "cn=wang wu,ou=developers,dc=example,dc=org",
    password: "password789",
    name: "王五",
  },
];

async function testConnection() {
  console.log("🔐 LDAP Docker 连接测试\n");

  try {
    // 1. 测试管理员连接
    console.log("1️⃣ 测试管理员连接...");
    const adminClient = await createLdapClient(LDAP_CONFIG);
    console.log("   ✅ 管理员连接成功");

    // 2. 测试搜索所有用户
    console.log("\n2️⃣ 搜索所有用户...");
    const entries = await adminClient.search("ou=developers,dc=example,dc=org", {
      filter: "(objectClass=person)",
      scope: "sub",
    });

    console.log(`   ✅ 找到 ${entries.length} 个用户:`);
    entries.forEach((entry) => {
      console.log(`      - DN: ${entry.dn}`);
      if (entry.attributes.cn) {
        console.log(`        姓名: ${entry.attributes.cn[0]}`);
      }
      if (entry.attributes.mail) {
        console.log(`        邮箱: ${entry.attributes.mail[0]}`);
      }
      if (entry.attributes.uid) {
        console.log(`        UID: ${entry.attributes.uid[0]}`);
      }
    });

    await adminClient.close();

    // 3. 测试用户认证
    console.log("\n3️⃣ 测试用户认证...");
    for (const user of TEST_USERS) {
      try {
        const userClient = await createLdapClient({
          url: LDAP_CONFIG.url,
        });

        await userClient.bind(user.dn, user.password);
        console.log(`   ✅ ${user.name} (${user.dn}) 认证成功`);

        await userClient.close();
      } catch (error) {
        console.log(
          `   ❌ ${user.name} (${user.dn}) 认证失败: ${(error as Error).message}`
        );
      }
    }

    // 4. 测试搜索特定用户
    console.log("\n4️⃣ 搜索特定用户...");
    const searchClient = await createLdapClient(LDAP_CONFIG);

    const specificUser = await searchClient.search(
      "ou=developers,dc=example,dc=org",
      {
        filter: "(cn=zhang san)",
        scope: "sub",
      }
    );

    if (specificUser.length > 0) {
      console.log("   ✅ 找到用户:");
      console.log(`      DN: ${specificUser[0].dn}`);
      console.log(`      属性: ${JSON.stringify(specificUser[0].attributes, null, 8)}`);
    } else {
      console.log("   ❌ 未找到用户");
    }

    await searchClient.close();

    // 5. 测试匿名绑定（应该失败）
    console.log("\n5️⃣ 测试匿名绑定（应该失败）...");
    try {
      const anonClient = await createLdapClient({
        url: LDAP_CONFIG.url,
      });
      await anonClient.anonymousBind();
      console.log("   ⚠️  匿名绑定成功（可能未配置匿名访问限制）");
      await anonClient.close();
    } catch (error) {
      console.log(`   ✅ 匿名绑定被拒绝（符合预期）`);
    }

    console.log("\n✅ 所有测试完成！");
  } catch (error) {
    console.error("\n❌ 测试失败:", (error as Error).message);
    console.error("\n请确保:");
    console.error("1. Docker 容器正在运行: docker-compose up -d openldap");
    console.error("2. LDAP 服务已启动: docker-compose ps");
    console.error("3. 端口 1389 未被占用");
    Deno.exit(1);
  }
}

// 运行测试
if (import.meta.main) {
  await testConnection();
}
