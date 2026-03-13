/**
 * Redis Session E2E Tests
 *
 * Tests session sharing between multiple TSP instances via Redis
 */

import {
  printSubsection,
  printTestResult,
  COLORS,
  assertEquals,
  getTestRoot,
} from "../run_e2e_tests.ts";

// Use ports 19001 and 19002 for the two test servers
const TEST_PORT_1 = 19001;
const TEST_PORT_2 = 19002;

export function getRedisSessionTests() {
  return [
    {
      name: "redis-session - Session sharing between multiple TSP instances",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Redis Session Sharing Test");

        // Check if Redis container is running
        console.log(`  ${COLORS.dim}Checking Redis container status...${COLORS.reset}`);

        let redisRunning = false;
        try {
          const checkCommand = new Deno.Command("docker", {
            args: ["ps", "--filter", "name=tsp-redis", "--format", "{{.Status}}"],
            stdout: "piped",
            stderr: "piped",
          });

          const { stdout } = await checkCommand.output();
          const status = new TextDecoder().decode(stdout).trim();

          if (status.includes("Up")) {
            redisRunning = true;
            console.log(`  ${COLORS.green}✓ Redis container is running${COLORS.reset}`);
          }
        } catch (error) {
          console.log(
            `  ${COLORS.yellow}⚠ Cannot check Redis status, skipping test${COLORS.reset}`,
          );
          console.log(`  ${COLORS.dim}Error: ${(error as Error).message}${COLORS.reset}`);
        }

        if (!redisRunning) {
          console.log(
            `  ${COLORS.yellow}⚠ Redis container not running, skipping test${COLORS.reset}`,
          );
          return;
        }

        // Get test root - use absolute path from project root
        const cwd = Deno.cwd();
        const testRoot = cwd + "/tests/test_www";
        console.log(`  ${COLORS.dim}Test root: ${testRoot}${COLORS.reset}`);

        // Create config files in project directory (not temp) so paths resolve correctly
        const configPath = cwd + "/test_redis_session_config1.json";
        const configPath2 = cwd + "/test_redis_session_config2.json";

        const config1Content = JSON.stringify({
          root: testRoot,
          port: TEST_PORT_1,
          dev: true,
          redis: {
            host: "127.0.0.1",
            port: 6379,
            db: 15, // Use different DB to avoid conflicts with other tests
          },
        }, null, 2);

        const config2Content = JSON.stringify({
          root: testRoot,
          port: TEST_PORT_2,
          dev: true,
          redis: {
            host: "127.0.0.1",
            port: 6379,
            db: 15,
          },
        }, null, 2);

        await Deno.writeTextFile(configPath, config1Content);
        await Deno.writeTextFile(configPath2, config2Content);

        console.log(`    ${COLORS.dim}Config 1: ${config1Content}${COLORS.reset}`);

        // Start server 1
        console.log(`  ${COLORS.dim}Starting TSP server 1 on port ${TEST_PORT_1}...${COLORS.reset}`);

        const server1Process = new Deno.Command(Deno.execPath(), {
          args: [
            "run",
            "--allow-all",
            "src/main.ts",
            "--config",
            configPath,
          ],
          stdout: "piped",
          stderr: "piped",
        }).spawn();

        // Wait a bit and then read output
        await new Promise(resolve => setTimeout(resolve, 1000));

        // Wait for server 1 to start
        await new Promise(resolve => setTimeout(resolve, 3000));

        // Start server 2
        console.log(`  ${COLORS.dim}Starting TSP server 2 on port ${TEST_PORT_2}...${COLORS.reset}`);

        const server2Process = new Deno.Command(Deno.execPath(), {
          args: [
            "run",
            "--allow-all",
            "src/main.ts",
            "--config",
            configPath2,
          ],
          stdout: "piped",
          stderr: "piped",
        }).spawn();

        // Wait for server 2 to start
        await new Promise(resolve => setTimeout(resolve, 3000));

        try {
          // Test 1: Create session on server 1
          console.log(`  ${COLORS.dim}Test 1: Create session on server 1${COLORS.reset}`);

          // Check if server 1 is responding
          let server1Ready = false;
          for (let i = 0; i < 10; i++) {
            try {
              const testResponse = await fetch(`http://localhost:${TEST_PORT_1}/session_e2e.tsp`, { method: "GET" });
              if (testResponse.status === 200 || testResponse.status === 405) { // 405 = method not allowed but server is up
                server1Ready = true;
                console.log(`    ${COLORS.green}✓ Server 1 is ready (status: ${testResponse.status})${COLORS.reset}`);
                break;
              } else if (testResponse.status >= 400) {
                const errorText = await testResponse.text();
                // Try to extract error message
                const errorMatch = errorText.match(/<div class="error-msg">([\s\S]*?)<\/div>/);
                if (errorMatch) {
                  console.log(`    ${COLORS.dim}Server 1 error: ${errorMatch[1].substring(0, 300)}${COLORS.reset}`);
                } else {
                  console.log(`    ${COLORS.dim}Server 1 error (${testResponse.status}): ${errorText.substring(0, 200)}${COLORS.reset}`);
                }
              }
            } catch (error) {
              console.log(`    ${COLORS.dim}Connection error: ${(error as Error).message}${COLORS.reset}`);
            }
            await new Promise(resolve => setTimeout(resolve, 500));
          }

          if (!server1Ready) {
            throw new Error("Server 1 failed to start");
          }

          // Check if server 2 is responding
          let server2Ready = false;
          for (let i = 0; i < 10; i++) {
            try {
              const testResponse = await fetch(`http://localhost:${TEST_PORT_2}/session_e2e.tsp`, { method: "GET" });
              if (testResponse.status === 200 || testResponse.status === 405) {
                server2Ready = true;
                console.log(`    ${COLORS.green}✓ Server 2 is ready (status: ${testResponse.status})${COLORS.reset}`);
                break;
              }
            } catch {
              // Server not ready yet
            }
            await new Promise(resolve => setTimeout(resolve, 500));
          }

          if (!server2Ready) {
            throw new Error("Server 2 failed to start");
          }

          const jar = new CookieJar();

          // Initialize session on server 1
          const initResponse = await fetch(
            `http://localhost:${TEST_PORT_1}/session_e2e.tsp`,
            {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ action: "init" }),
            },
          );

          console.log(`    ${COLORS.dim}Init response status: ${initResponse.status}${COLORS.reset}`);

          if (initResponse.status !== 200) {
            const text = await initResponse.text();
            throw new Error(`Server 1 returned ${initResponse.status}: ${text.substring(0, 200)}`);
          }

          assertEquals(initResponse.status, 200);

          // Get cookies from response
          const setCookieHeader = initResponse.headers.get("set-cookie");
          if (setCookieHeader) {
            const parts = setCookieHeader.split(";")[0];
            const [name, ...valueParts] = parts.split("=");
            const value = valueParts.join("=");
            if (name && value) {
              jar.set(name, value);
            }
          }

          console.log(`    ${COLORS.dim}Session cookies: ${JSON.stringify(jar.cookies)}${COLORS.reset}`);

          printTestResult("Create session on server 1", true);

          // Test 2: Read session on server 2 using same cookie
          console.log(`  ${COLORS.dim}Test 2: Read session on server 2${COLORS.reset}`);

          // Build cookie header
          const cookieHeader = Object.entries(jar.cookies)
            .map(([k, v]) => `${k}=${v}`)
            .join("; ");

          const getResponse = await fetch(
            `http://localhost:${TEST_PORT_2}/session_e2e.tsp`,
            {
              headers: {
                "Cookie": cookieHeader,
              },
            },
          );

          assertEquals(getResponse.status, 200);

          // Parse response - extract JSON from <div>...</div>
          const text = await getResponse.text();
          const jsonMatch = text.match(/<div>([\s\S]*?)<\/div>/);

          if (!jsonMatch) {
            throw new Error("Cannot extract JSON data from response");
          }

          // HTML entity decode
          const jsonString = jsonMatch[1]
            .replace(/&quot;/g, '"')
            .replace(/&amp;/g, "&")
            .replace(/&lt;/g, "<")
            .replace(/&gt;/g, ">");

          const data = JSON.parse(jsonString);

          // Verify session data is shared
          if (data.getData?.userId !== "e2e-test-user") {
            throw new Error(
              `Session not shared! Expected userId=e2e-test-user, got: ${JSON.stringify(data.getData)}`,
            );
          }

          if (data.getData?.userName !== "E2E Test User") {
            throw new Error(
              `Session not shared! Expected userName=E2E Test User, got: ${JSON.stringify(data.getData)}`,
            );
          }

          console.log(`    ${COLORS.dim}Session data from server 2: ${JSON.stringify(data.getData)}${COLORS.reset}`);

          printTestResult("Read session on server 2", true);

          // Test 3: Update session on server 2
          console.log(`  ${COLORS.dim}Test 3: Update session on server 2${COLORS.reset}`);

          const updateResponse = await fetch(
            `http://localhost:${TEST_PORT_2}/session_e2e.tsp`,
            {
              method: "POST",
              headers: {
                "Content-Type": "application/json",
                "Cookie": cookieHeader,
              },
              body: JSON.stringify({ action: "update" }),
            },
          );

          assertEquals(updateResponse.status, 200);

          // Test 4: Verify update on server 1
          console.log(`  ${COLORS.dim}Test 4: Verify update on server 1${COLORS.reset}`);

          const verifyResponse = await fetch(
            `http://localhost:${TEST_PORT_1}/session_e2e.tsp`,
            {
              headers: {
                "Cookie": cookieHeader,
              },
            },
          );

          assertEquals(verifyResponse.status, 200);

          // Parse response
          const verifyText = await verifyResponse.text();
          const verifyJsonMatch = verifyText.match(/<div>([\s\S]*?)<\/div>/);

          if (!verifyJsonMatch) {
            throw new Error("Cannot extract JSON data from verify response");
          }

          const verifyJsonString = verifyJsonMatch[1]
            .replace(/&quot;/g, '"')
            .replace(/&amp;/g, "&")
            .replace(/&lt;/g, "<")
            .replace(/&gt;/g, ">");

          const verifyData = JSON.parse(verifyJsonString);

          if (verifyData.getData?.updatedValue !== "updated-on-server-2") {
            throw new Error(
              `Session update not propagated! Expected updatedValue=updated-on-server-2, got: ${JSON.stringify(verifyData.getData)}`,
            );
          }

          console.log(`    ${COLORS.dim}Updated session data: ${JSON.stringify(verifyData.getData)}${COLORS.reset}`);

          printTestResult("Update and verify session", true);

          console.log(`  ${COLORS.green}✓ Redis session sharing test passed!${COLORS.reset}`);

        } finally {
          // Cleanup: kill servers
          console.log(`  ${COLORS.dim}Cleaning up...${COLORS.reset}`);

          server1Process.kill();
          server2Process.kill();

          // Cleanup config files
          try {
            await Deno.remove(configPath);
            await Deno.remove(configPath2);
          } catch {
            // Ignore cleanup errors
          }

          // Wait for processes to exit
          await new Promise(resolve => setTimeout(resolve, 500));
        }

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}

/**
 * Simple cookie jar for testing
 */
class CookieJar {
  cookies: Record<string, string> = {};

  set(name: string, value: string): void {
    this.cookies[name] = value;
  }

  get(name: string): string | undefined {
    return this.cookies[name];
  }
}
