/**
 * E2E 测试共享工具
 */

const OUTPUT_BINARY = "tspserver-test";
const TEST_PORT = 9001;
const TEST_ROOT = "./tests/test_www";
const STARTUP_DELAY = 2000;

// 全局服务器进程
let serverProcess: Deno.ChildProcess | null = null;

/**
 * 获取二进制文件路径
 */
export function getBinaryPath(): string {
  return Deno.build.os === "windows" ? `${OUTPUT_BINARY}.exe` : OUTPUT_BINARY;
}

/**
 * 编译二进制文件
 */
export async function compileBinary(): Promise<void> {
  const outputFile = getBinaryPath();

  const command = new Deno.Command("deno", {
    args: [
      "compile",
      "--allow-net",
      "--allow-read",
      "--allow-write",
      "--allow-env",
      "--output",
      outputFile,
      "src/main.ts",
    ],
    stdout: "piped",
    stderr: "piped",
  });

  const { code, stderr } = await command.output();

  if (code !== 0) {
    const stderrText = new TextDecoder().decode(stderr);
    throw new Error(`编译失败: ${stderrText}`);
  }
}

/**
 * 启动服务器进程
 */
export async function startServer(args: string[] = []): Promise<void> {
  const binaryPath = getBinaryPath();
  const commandPath = Deno.build.os === "windows" && !binaryPath.startsWith("./")
    ? `./${binaryPath}`
    : binaryPath;

  const command = new Deno.Command(commandPath, {
    args: ["--root", TEST_ROOT, "--port", TEST_PORT.toString(), ...args],
    stdout: "piped",
    stderr: "piped",
  });

  serverProcess = command.spawn();

  // 等待服务器启动
  await new Promise((resolve) => setTimeout(resolve, STARTUP_DELAY));
}

/**
 * 停止服务器进程
 */
export async function stopServer(): Promise<void> {
  if (serverProcess) {
    try {
      serverProcess.kill("SIGTERM");
    } catch {
      // 忽略错误
    }
    serverProcess = null;
  }
}

/**
 * 清理旧的二进制文件
 */
export async function cleanupBinary(): Promise<void> {
  const filesToRemove = [OUTPUT_BINARY];
  if (Deno.build.os === "windows") {
    filesToRemove.push(`${OUTPUT_BINARY}.exe`);
  }

  for (const file of filesToRemove) {
    try {
      await Deno.remove(file);
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        // 文件不存在，忽略
      } else {
        throw error;
      }
    }
  }
}

/**
 * 测试 HTTP 请求
 */
export async function testHttpRequest(
  url: string,
  expectedStatus: number,
  options: {
    expectedContentType?: string;
    expectHtml?: boolean;
    method?: "GET" | "POST" | "PUT" | "DELETE";
    body?: string;
    headers?: Record<string, string>;
  } = {}
): Promise<void> {
  const {
    expectedContentType,
    expectHtml = false,
    method = "GET",
    body,
    headers,
  } = options;

  const response = await fetch(url, {
    method,
    body,
    headers,
  });

  if (response.status !== expectedStatus) {
    throw new Error(`期望状态码 ${expectedStatus}，实际得到 ${response.status}`);
  }

  const text = await response.text();

  // 检查 Content-Type（如果指定了期望值）
  if (expectedContentType) {
    const contentType = response.headers.get("content-type");
    const hasContentType = contentType?.includes(expectedContentType);
    if (!hasContentType) {
      throw new Error(`期望 Content-Type 包含 ${expectedContentType}，实际得到 ${contentType}`);
    }
  }

  // 对于 HTML 响应，检查是否包含 HTML 标签
  if (expectHtml && expectedStatus === 200) {
    const hasHtml = text.includes("<html") || text.includes("<!DOCTYPE");
    if (!hasHtml) {
      throw new Error("响应不包含 HTML 标签");
    }
  }
}

/**
 * 杀掉占用端口的进程
 */
export async function killProcessOnPort(port: number): Promise<void> {
  try {
    let pids: number[] = [];

    if (Deno.build.os === "windows") {
      const netstatCommand = new Deno.Command("netstat", {
        args: ["-ano"],
        stdout: "piped",
        stderr: "piped",
      });

      const { stdout } = await netstatCommand.output();
      const output = new TextDecoder().decode(stdout);

      const lines = output.split("\n");
      for (const line of lines) {
        if (line.includes(`:${port}`) && line.includes("LISTENING")) {
          const parts = line.trim().split(/\s+/);
          const pid = parseInt(parts[parts.length - 1]);
          if (!isNaN(pid)) {
            pids.push(pid);
          }
        }
      }
    } else {
      const lsofCommand = new Deno.Command("lsof", {
        args: ["-ti", `:${port}`],
        stdout: "piped",
        stderr: "piped",
      });

      const { stdout, code } = await lsofCommand.output();

      if (code === 0) {
        const output = new TextDecoder().decode(stdout);
        const pidsStr = output.trim().split("\n");
        pids = pidsStr.map((pid) => parseInt(pid)).filter((pid) => !isNaN(pid));
      }
    }

    if (pids.length > 0) {
      for (const pid of pids) {
        try {
          if (Deno.build.os === "windows") {
            const killCommand = new Deno.Command("taskkill", {
              args: ["/PID", pid.toString(), "/F"],
              stdout: "piped",
              stderr: "piped",
            });
            await killCommand.output();
          } else {
            Deno.kill(pid, "SIGKILL");
          }
        } catch {
          // 忽略终止失败
        }
      }

      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  } catch {
    // 忽略端口检查错误
  }
}

export { OUTPUT_BINARY, TEST_PORT, TEST_ROOT, STARTUP_DELAY, serverProcess };
