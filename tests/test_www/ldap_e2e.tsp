/**
 * LDAP E2E Test Page - TSP File
 * Used to test LDAP client various features
 */

export default Page(async function (ctx, { createLdap, response, query, createZod }) {
  const z = await createZod();
  const { action = 'connect' } = query(z.object({
    action: z.string().default('connect')
  }));

  // LDAP config (Samba AD)
  const ldapConfig = {
    url: "ldap://localhost:389",
    bindDN: "CN=Administrator,CN=Users,DC=example,DC=com",
    bindCredentials: "P@ssw0rd123",
    baseDN: "DC=example,DC=com",
  };

  try {
    switch (action) {
      case "connect": {
        // Test connection
        const ldap = await createLdap(ldapConfig);

        return response.json({
          success: true,
          action: "connect",
          message: "LDAP connection successful",
          config: {
            url: ldapConfig.url,
            baseDN: ldapConfig.baseDN,
          },
        });
      }

      case "search": {
        // Test search - search test users in Samba AD
        const ldap = await createLdap(ldapConfig);

        const entries = await ldap.search("CN=Users,DC=example,DC=com", {
          filter: "(objectClass=user)",
          scope: "sub",
        });

        await ldap.close();

        // Filter out test users
        const testUsers = entries.filter(entry =>
          entry.dn.includes("CN=t_") || entry.dn.includes("CN=Administrator")
        );

        return response.json({
          success: true,
          action: "search",
          count: testUsers.length,
          entries: testUsers.map((entry) => ({
            dn: entry.dn,
            cn: (entry.attributes && entry.attributes.cn && entry.attributes.cn[0]) || "",
            mail: (entry.attributes && entry.attributes.mail && entry.attributes.mail[0]) || "",
            displayName: (entry.attributes && entry.attributes.displayName && entry.attributes.displayName[0]) || "",
          })),
        });
      }

      case "search-specific": {
        // Test search for specific user
        const ldap = await createLdap(ldapConfig);

        const entries = await ldap.search("CN=Users,DC=example,DC=com", {
          filter: "(cn=t_zhangw)",
          scope: "sub",
        });

        await ldap.close();

        if (entries.length === 0) {
          return response.json({
            success: false,
            action: "search-specific",
            error: "User t_zhangw not found",
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
        // Test user authentication
        const { userDN, password } = ctx.body as {
          userDN: string;
          password: string;
        };

        // Create new LDAP connection without admin credentials
        const ldap = await createLdap({
          url: ldapConfig.url,
        });

        try {
          // Try to bind with user credentials
          await ldap.bind(userDN, password);
          await ldap.close();

          return response.json({
            success: true,
            action: "authenticate",
            message: "Authentication successful",
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
        // Batch test user authentication (Samba AD test users)
        const testUsers = [
          {
            dn: "CN=t_zhangw,CN=Users,DC=example,DC=com",
            password: "Test@123",
            name: "t_zhangw",
          },
          {
            dn: "CN=t_lina,CN=Users,DC=example,DC=com",
            password: "Test@123",
            name: "t_lina",
          },
          {
            dn: "CN=t_wangq,CN=Users,DC=example,DC=com",
            password: "Test@123",
            name: "t_wangq",
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
        // Test compare operation
        const ldap = await createLdap(ldapConfig);

        const result = await ldap.compare(
          "CN=t_zhangw,CN=Users,DC=example,DC=com",
          "mail",
          "t_zhangw@cnnp.com.cn"
        );

        await ldap.close();

        return response.json({
          success: true,
          action: "compare",
          matches: result,
        });
      }

      case "add": {
        // Samba AD does not support creating users via ldap.add (need to use samba-tool)
        // Skip this test
        return response.json({
          success: true,
          action: "add",
          message: "Samba AD requires samba-tool to create users, skipping this test",
        });
      }

      case "modify": {
        // Test modify entry (modify t_zhangw's description attribute)
        const ldap = await createLdap(ldapConfig);

        const timestamp = Date.now().toString();
        const testValue = `E2E Test ${timestamp}`;

        await ldap.modify(
          "CN=t_zhangw,CN=Users,DC=example,DC=com",
          [
            {
              operation: "replace",
              modification: { description: testValue },
            },
          ]
        );

        // Restore original value
        await ldap.modify(
          "CN=t_zhangw,CN=Users,DC=example,DC=com",
          [
            {
              operation: "replace",
              modification: { description: "Test user 1" },
            },
          ]
        );

        await ldap.close();

        return response.json({
          success: true,
          action: "modify",
          message: "Modify successful and restored",
        });
      }

      case "delete": {
        // Samba AD does not support deleting users via normal LDAP operations (need to use samba-tool)
        // Skip this test
        return response.json({
          success: true,
          action: "delete",
          message: "Samba AD requires samba-tool to delete users, skipping this test",
        });
      }

      case "anonymous": {
        // Test anonymous bind (should fail)
        const ldap = await createLdap({
          url: ldapConfig.url,
        });

        try {
          await ldap.anonymousBind();
          await ldap.close();

          return response.json({
            success: true,
            action: "anonymous",
            message: "Anonymous bind successful (may not be configured restrictions)",
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
        // Stress test - multiple search operations
        const ldap = await createLdap(ldapConfig);

        const iterations = 50;
        const startTime = Date.now();

        for (let i = 0; i < iterations; i++) {
          await ldap.search("CN=Users,DC=example,DC=com", {
            filter: "(objectClass=user)",
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
          error: "Unknown action",
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
