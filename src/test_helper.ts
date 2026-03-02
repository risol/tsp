/**
 * TSP built-in test helper module
 *
 * Provides users with Jest/PHPUnit-like testing functionality
 * No additional tools needed, write tests directly in TSX
 */

/**
 * Test result type
 */
export interface TestResult {
  /** Test name */
  name: string;
  /** Whether passed */
  passed: boolean;
  /** Error message */
  error?: string;
  /** Test data */
  data?: unknown;
  /** Execution time in milliseconds */
  duration?: number;
}

/**
 * Test suite result
 */
export interface TestSuiteResult {
  /** Total test count */
  total: number;
  /** Passed count */
  passed: number;
  /** Failed count */
  failed: number;
  /** Test results list */
  results: TestResult[];
  /** Total execution time in milliseconds */
  duration: number;
}

/**
 * Mocked PageContext
 * Used to mock HTTP request context during testing
 */
export interface MockPageContext {
  /** HTTP method */
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
  /** Request URL (automatically converted to URL object) */
  url: string | URL;
  /** Request headers */
  headers?: Record<string, string>;
  /** URL query parameters */
  query?: Record<string, string>;
  /** Request body data */
  body?: unknown;
  /** Cookies */
  cookies?: Record<string, string>;
}

/**
 * Mocked Response object
 */
export interface MockResponse {
  /** Return JSON response */
  json(data: unknown, status?: number): Response;
  /** Return HTML response */
  html(html: string, status?: number): Response;
  /** Return text response */
  text(text: string, status?: number): Response;
  /** Return redirect */
  redirect(url: string, status?: 301 | 302 | 303 | 307 | 308): Response;
}

/**
 * HTTP request mock options
 */
export interface MockFetchOptions {
  /** Response data */
  response: unknown;
  /** Response status code */
  status?: number;
  /** Response headers */
  headers?: Record<string, string>;
  /** Response delay in milliseconds */
  delay?: number;
}

/**
 * TSP test helper
 */
export interface TestHelper {
  /**
   * Run a single test
   * @param name - Test name
   * @param fn - Test function (can be sync or async)
   */
  test(name: string, fn: () => void | Promise<void>): Promise<TestResult>;

  /**
   * Assert equal
   * @param actual - Actual value
   * @param expected - Expected value
   * @param message - Error message
   */
  assertEqual(actual: unknown, expected: unknown, message?: string): void;

  /**
   * Assert not equal
   * @param actual - Actual value
   * @param expected - Expected value
   * @param message - Error message
   */
  assertNotEqual(actual: unknown, expected: unknown, message?: string): void;

  /**
   * Assert true
   * @param value - Value
   * @param message - Error message
   */
  assertTrue(value: unknown, message?: string): void;

  /**
   * Assert false
   * @param value - Value
   * @param message - Error message
   */
  assertFalse(value: unknown, message?: string): void;

  /**
   * Assert null
   * @param value - Value
   * @param message - Error message
   */
  assertNull(value: unknown, message?: string): void;

  /**
   * Assert not null
   * @param value - Value
   * @param message - Error message
   */
  assertNotNull(value: unknown, message?: string): void;

  /**
   * Assert contains
   * @param haystack - String or array
   * @param needle - Value to find
   * @param message - Error message
   */
  assertContains(haystack: string | unknown[], needle: unknown, message?: string): void;

  /**
   * Assert throws
   * @param fn - Function
   * @param expectedError - Expected error type or message
   * @param message - Error message
   */
  assertThrows(fn: () => void, expectedError?: string | RegExp, message?: string): void;

  /**
   * Directly fail the test
   * @param message - Failure reason
   */
  fail(message?: string): void;

  /**
   * NEW: Mock PageContext
   * Create a mocked page context for direct testing of page functions
   *
   * @example
   * ```tsx
   * const mockCtx = testHelper.mockContext({
   *   method: 'POST',
   *   url: 'http://localhost:9000/api/user',
   *   query: { id: '123' },
   *   body: { name: 'Alice', age: 25 }
   * });
   * ```
   */
  mockContext(ctx: MockPageContext): PageContext;

  /**
   * NEW: Directly call page function (unit test)
   * Call and test page function directly without HTTP layer
   *
   * @example
   * ```tsx
   * import userPage from './user.tsx';
   *
   * const mockCtx = testHelper.mockContext({
   *   method: 'GET',
   *   url: 'http://localhost:9000/user/123'
   * });
   *
   * const result = await testHelper.runPage(userPage, mockCtx);
   * testHelper.assertEqual(result.status, 200);
   * ```
   */
  runPage(pageFn: (ctx: PageContext, deps: Record<string, unknown>) => Promise<unknown>, ctx: PageContext): Promise<unknown>;

  /**
   * NEW: Mock HTTP request
   * Intercept and mock fetch requests to avoid real network calls
   *
   * @param urlPattern - URL match pattern (string or regex)
   * @param options - Mock options
   *
   * @example
   * ```tsx
   * // Mock specific URL
   * testHelper.mockFetch('/api/users', {
   *   response: [{ id: 1, name: 'Alice' }],
   *   status: 200
   * });
   *
   * // Mock regex matching
   * testHelper.mockFetch(/\/api\/users\/\d+/, {
   *   response: { id: 1, name: 'Alice' }
   * });
   * ```
   */
  mockFetch(urlPattern: string | RegExp, options: MockFetchOptions): void;

  /**
   * NEW: Clear all mocks
   * Clear all fetch mocks, restore real network calls
   */
  clearMocks(): void;

  /**
   * NEW: Snapshot test
   * Compare value with snapshot file for UI regression testing
   *
   * @param name - Snapshot name
   * @param value - Value to snapshot
   * @param update - Whether to update snapshot (default false)
   *
   * @example
   * ```tsx
   * testHelper.assertSnapshot('user-list', users);
   * ```
   */
  assertSnapshot(name: string, value: unknown, update?: boolean): Promise<void>;

  /**
   * Get all test results
   */
  getResults(): TestSuiteResult;

  /**
   * Clear test results
   */
  clear(): void;

  /**
   * Generate HTML format test report
   */
  toHTML(): string;

  /**
   * Generate JSON format test report
   */
  toJSON(): string;
}

/**
 * Test helper implementation
 */
class TestHelperImpl implements TestHelper {
  private results: TestResult[] = [];
  private startTime: number = 0;
  private fetchMocks: Array<{ pattern: string | RegExp; options: MockFetchOptions }> = [];
  private originalFetch: typeof fetch | null = null;

  async test(name: string, fn: () => void | Promise<void>): Promise<TestResult> {
    const startTime = Date.now();
    let result: TestResult;

    try {
      await fn();
      result = {
        name,
        passed: true,
        duration: Date.now() - startTime
      };
    } catch (error) {
      result = {
        name,
        passed: false,
        error: error instanceof Error ? error.message : String(error),
        duration: Date.now() - startTime
      };
    }

    this.results.push(result);
    return result;
  }

  assertEqual(actual: unknown, expected: unknown, message?: string): void {
    if (actual !== expected) {
      throw new Error(
        message || `Expected ${JSON.stringify(expected)}, but got ${JSON.stringify(actual)}`
      );
    }
  }

  assertNotEqual(actual: unknown, expected: unknown, message?: string): void {
    if (actual === expected) {
      throw new Error(
        message || `Expected ${JSON.stringify(actual)} to not equal ${JSON.stringify(expected)}`
      );
    }
  }

  assertTrue(value: unknown, message?: string): void {
    if (!value) {
      throw new Error(message || `Expected truthy value, but got ${JSON.stringify(value)}`);
    }
  }

  assertFalse(value: unknown, message?: string): void {
    if (value) {
      throw new Error(message || `Expected falsy value, but got ${JSON.stringify(value)}`);
    }
  }

  assertNull(value: unknown, message?: string): void {
    if (value !== null) {
      throw new Error(message || `Expected null, but got ${JSON.stringify(value)}`);
    }
  }

  assertNotNull(value: unknown, message?: string): void {
    if (value === null) {
      throw new Error(message || 'Expected not null');
    }
  }

  assertContains(haystack: string | unknown[], needle: unknown, message?: string): void {
    let found = false;

    if (typeof haystack === 'string') {
      found = haystack.includes(String(needle));
    } else if (Array.isArray(haystack)) {
      found = haystack.includes(needle);
    }

    if (!found) {
      throw new Error(
        message || `Expected ${JSON.stringify(haystack)} to contain ${JSON.stringify(needle)}`
      );
    }
  }

  assertThrows(fn: () => void, expectedError?: string | RegExp, message?: string): void {
    try {
      fn();
      throw new Error(message || 'Expected function to throw an error, but it did not');
    } catch (error) {
      if (expectedError) {
        const errorMsg = error instanceof Error ? error.message : String(error);

        if (expectedError instanceof RegExp) {
          if (!expectedError.test(errorMsg)) {
            throw new Error(
              message || `Expected error message to match ${expectedError}, but got "${errorMsg}"`
            );
          }
        } else {
          if (!errorMsg.includes(expectedError)) {
            throw new Error(
              message || `Expected error message to contain "${expectedError}", but got "${errorMsg}"`
            );
          }
        }
      }
    }
  }

  /**
   * Directly fail the test
   * @param message - Failure reason
   */
  fail(message?: string): void {
    throw new Error(message || 'Test failed');
  }

  /**
   * IMPLEMENTATION: Mock PageContext
   * Note: The returned object does not contain query and body, forcing use of dependency injection
   */
  mockContext(ctx: MockPageContext): PageContext {
    const url = typeof ctx.url === 'string' ? new URL(ctx.url) : ctx.url;
    const headers = new Headers(ctx.headers);

    return {
      method: ctx.method,
      url,
      headers,
      cookies: ctx.cookies || {},
      files: {},
      file: '/mock/file.tsx',
      root: '/mock/root'
    } as PageContext;
  }

  /**
   * IMPLEMENTATION: Directly call page function
   */
  async runPage(
    pageFn: (ctx: PageContext, deps: Record<string, unknown>) => Promise<unknown>,
    ctx: PageContext
  ): Promise<unknown> {
    // Create mock dependency injection
    const mockDeps = {
      response: {
        json: (data: unknown, status: number = 200) => {
          return new Response(JSON.stringify(data), {
            status,
            headers: { 'Content-Type': 'application/json' }
          });
        },
        html: (html: string, status: number = 200) => {
          return new Response(html, {
            status,
            headers: { 'Content-Type': 'text/html; charset=utf-8' }
          });
        },
        text: (text: string, status: number = 200) => {
          return new Response(text, {
            status,
            headers: { 'Content-Type': 'text/plain; charset=utf-8' }
          });
        },
        redirect: (url: string, status: number = 302) => {
          return new Response(null, {
            status,
            headers: { Location: url }
          });
        }
      }
    };

    return await pageFn(ctx, mockDeps);
  }

  /**
   * IMPLEMENTATION: Mock HTTP request
   */
  mockFetch(urlPattern: string | RegExp, options: MockFetchOptions): void {
    // Save original fetch
    if (this.originalFetch === null) {
      this.originalFetch = globalThis.fetch;
    }

    // Add mock rule
    this.fetchMocks.push({ pattern: urlPattern, options });

    // Replace global fetch
    globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;

      // Find matching mock
      for (const mock of this.fetchMocks) {
        const matches = typeof mock.pattern === 'string'
          ? url.includes(mock.pattern)
          : mock.pattern.test(url);

        if (matches) {
          // Delay response
          if (mock.options.delay) {
            await new Promise(resolve => setTimeout(resolve, mock.options.delay));
          }

          // Return mock response
          return new Response(JSON.stringify(mock.options.response), {
            status: mock.options.status || 200,
            headers: {
              'Content-Type': 'application/json',
              ...(mock.options.headers || {})
            }
          });
        }
      }

      // No matching mock, use original fetch
      if (this.originalFetch) {
        return this.originalFetch(input, init);
      }

      throw new Error(`No mock found for URL: ${url}`);
    }) as typeof fetch;
  }

  /**
   * IMPLEMENTATION: Clear all mocks
   */
  clearMocks(): void {
    if (this.originalFetch) {
      globalThis.fetch = this.originalFetch;
      this.originalFetch = null;
    }
    this.fetchMocks = [];
  }

  /**
   * IMPLEMENTATION: Snapshot test
   */
  async assertSnapshot(name: string, value: unknown, update: boolean = false): Promise<void> {
    const snapshotDir = '.tests/snapshots';
    const snapshotFile = `${snapshotDir}/${name}.snap.json`;

    // Ensure snapshot directory exists
    try {
      await Deno.mkdir(snapshotDir, { recursive: true });
    } catch {
      // Directory may already exist
    }

    const expectedValue = JSON.stringify(value, null, 2);

    if (update) {
      // Update mode: write new snapshot
      await Deno.writeTextFile(snapshotFile, expectedValue);
      return;
    }

    try {
      // Read snapshot file
      const snapshot = await Deno.readTextFile(snapshotFile);

      if (snapshot !== expectedValue) {
        throw new Error(
          `Snapshot "${name}" does not match\n` +
          `Expected:\n${snapshot}\n\n` +
          `Received:\n${expectedValue}\n\n` +
          `Tip: Use testHelper.assertSnapshot("${name}", value, true) to update the snapshot.`
        );
      }
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        // Snapshot doesn't exist, create new snapshot
        await Deno.writeTextFile(snapshotFile, expectedValue);
        throw new Error(
          `Snapshot "${name}" created. Run test again to verify.`
        );
      }
      throw error;
    }
  }

  toHTML(): string {
    const suite = this.getResults();
    const { passed, failed, total, duration, results } = suite;

    const statusClass = failed === 0 ? 'success' : 'warning';
    const statusIcon = failed === 0 ? '✓' : '✗';

    return `
<div class="test-suite" style="font-family: monospace;">
  <div class="test-summary alert alert-${statusClass}" role="alert">
    <h4 class="alert-heading">${statusIcon} Test Results: ${passed}/${total} Passed</h4>
    <hr>
    <div class="row">
      <div class="col">
        <strong>Total:</strong> ${total}
      </div>
      <div class="col">
        <strong class="text-success">Passed:</strong> ${passed}
      </div>
      <div class="col">
        <strong class="text-danger">Failed:</strong> ${failed}
      </div>
      <div class="col">
        <strong>Duration:</strong> ${duration}ms
      </div>
    </div>
  </div>

  <div class="test-results">
    ${results.map(result => `
      <div class="test-result mb-2 p-3 rounded ${result.passed ? 'bg-success bg-opacity-10' : 'bg-danger bg-opacity-10'}">
        <div class="d-flex justify-content-between align-items-center">
          <strong>${result.passed ? '✓' : '✗'} ${result.name}</strong>
          ${result.duration ? `<span class="badge bg-secondary">${result.duration}ms</span>` : ''}
        </div>
        ${result.error ? `<pre class="mt-2 mb-0 text-danger small">${result.error}</pre>` : ''}
        ${result.data ? `<pre class="mt-2 mb-0 small">${JSON.stringify(result.data, null, 2)}</pre>` : ''}
      </div>
    `).join('')}
  </div>
</div>
    `.trim();
  }

  toJSON(): string {
    return JSON.stringify(this.getResults(), null, 2);
  }

  getResults(): TestSuiteResult {
    return {
      total: this.results.length,
      passed: this.results.filter(r => r.passed).length,
      failed: this.results.filter(r => !r.passed).length,
      results: this.results,
      duration: this.results.reduce((sum, r) => sum + (r.duration || 0), 0)
    };
  }

  clear(): void {
    this.results = [];
    this.startTime = Date.now();
    this.clearMocks();
  }
}

/**
 * Create test helper instance
 */
export function createTestHelper(): TestHelper {
  return new TestHelperImpl();
}

/**
 * Default export
 */
export default {
  createTestHelper
};
