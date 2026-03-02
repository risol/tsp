/**
 * Access Log module unit tests
 * Tests logging in src/main.ts
 */

import {
  assertEquals,
  assertExists,
  assertStringIncludes,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { type Config, logAccess } from "../../src/main.ts";

// Temporary log file paths for testing
const TEST_LOG_FILE = "./test_access.log";
const TEST_LOG_FILE_2 = "./test_access_2.log";

/**
 * Cleanup test log files
 */
async function cleanupTestLog(filepath: string) {
  try {
    await Deno.remove(filepath);
  } catch {
    // File does not exist, ignore
  }
}

Deno.test("access log - logAccess: console output (default config)", async () => {
  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    // Not setting accessLogPath, should output to console
  };

  const request = new Request("http://localhost:9000/test/path?param=value", {
    method: "GET",
    headers: {
      "User-Agent": "TestAgent/1.0",
    },
  });

  const response = new Response("OK", { status: 200 });

  // This test mainly verifies no errors are thrown
  await logAccess(request, response, config);

  // If accessLogPath is not configured, log should go to console
  // We can't directly test console output but can verify function doesn't throw error
  assertEquals(true, true);
});

Deno.test("access log - logAccess: file output - basic request", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const request = new Request("http://localhost:9000/test/path?param=value", {
    method: "GET",
    headers: {
      "User-Agent": "TestAgent/1.0",
    },
  });

  const response = new Response("OK", { status: 200 });

  await logAccess(request, response, config);

  // Read log file
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);
  const logLines = logContent.trim().split("\n");

  // Verify log content
  assertEquals(logLines.length, 1);

  const logLine = logLines[0];
  assertStringIncludes(logLine, "GET");
  assertStringIncludes(logLine, "/test/path");
  assertStringIncludes(logLine, "200");
  assertStringIncludes(logLine, "TestAgent/1.0");

  // Verify ISO timestamp format (contains T and Z)
  assertStringIncludes(logLine, "T");
  assertStringIncludes(logLine, "Z");

  // Cleanup
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: file output - multiple appends", async () => {
  await cleanupTestLog(TEST_LOG_FILE_2);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE_2,
  };

  // Log three entries
  await logAccess(
    new Request("http://localhost:9000/page1", {
      method: "GET",
      headers: { "User-Agent": "Agent1" },
    }),
    new Response("OK", { status: 200 }),
    config,
  );

  await logAccess(
    new Request("http://localhost:9000/page2", {
      method: "POST",
      headers: { "User-Agent": "Agent2" },
    }),
    new Response("Created", { status: 201 }),
    config,
  );

  await logAccess(
    new Request("http://localhost:9000/page3", {
      method: "DELETE",
      headers: { "User-Agent": "Agent3" },
    }),
    new Response("Not Found", { status: 404 }),
    config,
  );

  // Read log file
  const logContent = await Deno.readTextFile(TEST_LOG_FILE_2);
  const logLines = logContent.trim().split("\n");

  // Verify three log lines
  assertEquals(logLines.length, 3);

  // Verify first line
  assertStringIncludes(logLines[0], "GET");
  assertStringIncludes(logLines[0], "/page1");
  assertStringIncludes(logLines[0], "200");
  assertStringIncludes(logLines[0], "Agent1");

  // Verify second line
  assertStringIncludes(logLines[1], "POST");
  assertStringIncludes(logLines[1], "/page2");
  assertStringIncludes(logLines[1], "201");
  assertStringIncludes(logLines[1], "Agent2");

  // Verify third line
  assertStringIncludes(logLines[2], "DELETE");
  assertStringIncludes(logLines[2], "/page3");
  assertStringIncludes(logLines[2], "404");
  assertStringIncludes(logLines[2], "Agent3");

  // Cleanup
  await cleanupTestLog(TEST_LOG_FILE_2);
});

Deno.test("access log - logAccess: no User-Agent", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const request = new Request("http://localhost:9000/test", {
    method: "GET",
    // Not setting User-Agent
  });

  const response = new Response("OK", { status: 200 });

  await logAccess(request, response, config);

  // Read log file
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);

  // Verify log contains "-" as User-Agent placeholder
  assertStringIncludes(logContent, '"-"');

  // Cleanup
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: various HTTP methods", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const methods = [
    "GET",
    "POST",
    "PUT",
    "PATCH",
    "DELETE",
    "HEAD",
    "OPTIONS",
  ] as const;

  for (const method of methods) {
    await logAccess(
      new Request("http://localhost:9000/test", {
        method,
        headers: { "User-Agent": "Test" },
      }),
      new Response("OK", { status: 200 }),
      config,
    );
  }

  // Read log file
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);
  const logLines = logContent.trim().split("\n");

  // Verify all methods are logged
  assertEquals(logLines.length, methods.length);

  for (let i = 0; i < methods.length; i++) {
    assertStringIncludes(logLines[i], methods[i]);
  }

  // Cleanup
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: various status codes", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const statusCodes = [200, 201, 301, 302, 400, 401, 403, 404, 500, 502, 503];

  for (const status of statusCodes) {
    await logAccess(
      new Request("http://localhost:9000/test", {
        method: "GET",
        headers: { "User-Agent": "Test" },
      }),
      new Response("Status", { status }),
      config,
    );
  }

  // Read log file
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);
  const logLines = logContent.trim().split("\n");

  // Verify all status codes are logged
  assertEquals(logLines.length, statusCodes.length);

  for (let i = 0; i < statusCodes.length; i++) {
    assertStringIncludes(logLines[i], statusCodes[i].toString());
  }

  // Cleanup
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: path contains query parameters", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const request = new Request(
    "http://localhost:9000/search?q=test&page=1&sort=desc",
    {
      method: "GET",
      headers: { "User-Agent": "Test" },
    },
  );

  const response = new Response("OK", { status: 200 });

  await logAccess(request, response, config);

  // Read log file
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);

  // Verify path is recorded (note: only pathname is logged, query parameters are not included)
  // According to current implementation, only pathname is recorded
  assertStringIncludes(logContent, "/search");

  // Cleanup
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: complex User-Agent", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const userAgent =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

  const request = new Request("http://localhost:9000/test", {
    method: "GET",
    headers: { "User-Agent": userAgent },
  });

  const response = new Response("OK", { status: 200 });

  await logAccess(request, response, config);

  // Read log file
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);

  // Verify complete User-Agent is logged
  assertStringIncludes(logContent, userAgent);

  // Cleanup
  await cleanupTestLog(TEST_LOG_FILE);
});

console.log("\n✓ Access Log module tests completed");
