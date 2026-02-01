/**
 * LDAP E2E 测试页面
 * 用于测试 LDAP 客户端的各种功能
 */

export default Page(async function (ctx, { createLdap, response }) {
  const action = ctx.query.action || "connect";

  // LDAP 配置
  const ldapConfig = {
    url: "ldap://localhost:1389",
    bindDN: "cn=admin,dc=example,dc=org",
    bindCredentials: "admin123456",
    baseDN: "dc=example,dc=org",
  };

  try {
    switch (action) {
      case "connect": {
        // 测试连接
        const ldap = await createLdap(ldapConfig);

        return response.json({
          success: true,
          action: "connect",
          message: "LDAP 连接成功",
          config: {
            url: ldapConfig.url,
            baseDN: ldapConfig.baseDN,
          },
        });
      }

      case "search": {
        // 测试搜索
        const ldap = await createLdap(ldapConfig);

        const entries = await ldap.search("ou=developers,dc=example,dc=org", {
          filter: "(objectClass=person)",
          scope: "sub",
        });

        await ldap.close();

        return response.json({
          success: true,
          action: "search",
          count: entries.length,
          entries: entries.map((entry) => ({
            dn: entry.dn,
            cn: (entry.attributes && entry.attributes.cn && entry.attributes.cn[0]) || "",
            mail: (entry.attributes && entry.attributes.mail && entry.attributes.mail[0]) || "",
            uid: (entry.attributes && entry.attributes.uid && entry.attributes.uid[0]) || "",
          })),
        });
      }

      case "search-specific": {
        // 测试搜索特定用户
        const ldap = await createLdap(ldapConfig);

        const entries = await ldap.search("ou=developers,dc=example,dc=org", {
          filter: "(cn=zhang san)",
          scope: "sub",
        });

        await ldap.close();

        if (entries.length === 0) {
          return response.json({
            success: false,
            action: "search-specific",
            error: "未找到用户",
          });
        }

        const entry = entries[0];
        return response.json({
          success: true,
          action: "search-specific",
          user: {
            dn: entry.dn,
            attributes: entry.attributes || {},
          },
        });
      }

      case "authenticate": {
        // 测试用户认证
        const { userDN, password } = ctx.body as {
          userDN: string;
          password: string;
        };

        // 创建新的 LDAP 连接，不使用管理员凭据
        const ldap = await createLdap({
          url: ldapConfig.url,
        });

        try {
          // 尝试使用用户凭据绑定
          await ldap.bind(userDN, password);
          await ldap.close();

          return response.json({
            success: true,
            action: "authenticate",
            message: "认证成功",
            userDN: userDN,
          });
        } catch (error) {
          await ldap.close();

          return response.json({
            success: false,
            action: "authenticate",
            error: (error as Error).message,
          });
        }
      }

      case "authenticate-users": {
        // 批量测试用户认证
        const testUsers = [
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

        const results = [];

        for (const user of testUsers) {
          const ldap = await createLdap({
            url: ldapConfig.url,
          });

          try {
            await ldap.bind(user.dn, user.password);
            await ldap.close();
            results.push({ user: user.name, success: true });
          } catch (error) {
            await ldap.close();
            results.push({
              user: user.name,
              success: false,
              error: (error as Error).message,
            });
          }
        }

        const allPassed = results.every((r) => r.success);

        return response.json({
          success: allPassed,
          action: "authenticate-users",
          results: results,
        });
      }

      case "compare": {
        // 测试比较操作
        const ldap = await createLdap(ldapConfig);

        const result = await ldap.compare(
          "cn=zhang san,ou=developers,dc=example,dc=org",
          "mail",
          "zhang.san@example.com"
        );

        await ldap.close();

        return response.json({
          success: true,
          action: "compare",
          matches: result,
        });
      }

      case "add": {
        // 测试添加条目
        const ldap = await createLdap(ldapConfig);

        const timestamp = Date.now();
        const dn = `cn=e2e_test_${timestamp},ou=developers,dc=example,dc=org`;

        await ldap.add(dn, {
          objectClass: ["inetOrgPerson", "organizationalPerson", "person"],
          cn: `e2e_test_${timestamp}`,
          sn: "test",
          mail: `e2e_test_${timestamp}@example.com`,
          userPassword: "test_password",
        });

        await ldap.close();

        return response.json({
          success: true,
          action: "add",
          dn: dn,
        });
      }

      case "modify": {
        // 测试修改条目
        const ldap = await createLdap(ldapConfig);

        await ldap.modify(
          "cn=zhang san,ou=developers,dc=example,dc=org",
          [
            {
              operation: "replace",
              modification: { mail: "updated@example.com" },
            },
          ]
        );

        // 恢复原值
        await ldap.modify(
          "cn=zhang san,ou=developers,dc=example,dc=org",
          [
            {
              operation: "replace",
              modification: { mail: "zhang.san@example.com" },
            },
          ]
        );

        await ldap.close();

        return response.json({
          success: true,
          action: "modify",
          message: "修改成功并已恢复",
        });
      }

      case "delete": {
        // 测试删除条目（先创建，再删除）
        const ldap = await createLdap(ldapConfig);

        const timestamp = Date.now();
        const dn = `cn=e2e_to_delete_${timestamp},ou=developers,dc=example,dc=org`;

        // 创建临时条目
        await ldap.add(dn, {
          objectClass: ["inetOrgPerson", "organizationalPerson", "person"],
          cn: `e2e_to_delete_${timestamp}`,
          sn: "delete_test",
          mail: `delete_test_${timestamp}@example.com`,
          userPassword: "test_password",
        });

        // 删除条目
        await ldap.del(dn);

        await ldap.close();

        return response.json({
          success: true,
          action: "delete",
          deletedDN: dn,
        });
      }

      case "anonymous": {
        // 测试匿名绑定（应该失败）
        const ldap = await createLdap({
          url: ldapConfig.url,
        });

        try {
          await ldap.anonymousBind();
          await ldap.close();

          return response.json({
            success: true,
            action: "anonymous",
            message: "匿名绑定成功（可能未配置限制）",
          });
        } catch (error) {
          await ldap.close();

          return response.json({
            success: false,
            action: "anonymous",
            error: (error as Error).message,
          });
        }
      }

      case "stress": {
        // 压力测试 - 多次搜索操作
        const ldap = await createLdap(ldapConfig);

        const iterations = 50;
        const startTime = Date.now();

        for (let i = 0; i < iterations; i++) {
          await ldap.search("ou=developers,dc=example,dc=org", {
            filter: "(objectClass=person)",
            scope: "sub",
          });
        }

        const duration = Date.now() - startTime;

        await ldap.close();

        return response.json({
          success: true,
          action: "stress",
          iterations: iterations,
          duration: duration,
          avgTime: duration / iterations,
        });
      }

      default: {
        return response.json({
          success: false,
          error: "未知的操作",
        });
      }
    }
  } catch (error) {
    return response.json({
      success: false,
      error: (error as Error).message,
      stack: (error as Error).stack,
    });
  }
});
