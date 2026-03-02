/**
 * TSP type definition module
 * Exports all business type definitions for dependency injection
 *
 * Note: These types are defined in types.d.ts in declare global
 * This file provides runtime implementation of type registry
 */

// ============================================
// Type references (from global declarations)
// ============================================

// These types are defined in types.d.ts in declare global
// Redeclared here to provide type hints
type HttpMethod = globalThis.HttpMethod;
type PageContext = globalThis.PageContext;
type RedirectResult = globalThis.RedirectResult;
type UploadedFile = globalThis.UploadedFile;
type ResponseHelper = globalThis.ResponseHelper;
type Logger = globalThis.Logger;
type LogRotationConfig = globalThis.LogRotationConfig;
type LoggerConfig = globalThis.LoggerConfig;
type FileManagerConfig = globalThis.FileManagerConfig;
type MySQLClient = globalThis.MySQLClient;
type MySQLConfig = globalThis.MySQLConfig;
type MySQLFactory = globalThis.MySQLFactory;
type RedisClient = globalThis.RedisClient;
type RedisConfig = globalThis.RedisConfig;
type RedisFactory = globalThis.RedisFactory;
type LdapClient = globalThis.LdapClient;
type LdapConfig = globalThis.LdapConfig;
type LdapEntry = globalThis.LdapEntry;
type ExcelJSFactory = globalThis.ExcelJSFactory;
type TestResult = globalThis.TestResult;
type TestSuiteResult = globalThis.TestSuiteResult;
type MockPageContext = globalThis.MockPageContext;
type MockResponse = globalThis.MockResponse;
type MockFetchOptions = globalThis.MockFetchOptions;
type TestHelper = globalThis.TestHelper;
type PageFunction<T> = globalThis.PageFunction<T>;
type Page<T> = globalThis.Page<T>;
type AppDeps = globalThis.AppDeps;

// ============================================
// Create type registry object (runtime metadata)
// ============================================

/**
 * Type registry
 * Provides type metadata at runtime
 *
 * Although these interfaces are empty objects at runtime (because types don't exist after compilation),
 * they provide complete type information for TypeScript
 */
export const createTypesRegistry = () => {
  return {
    // Type name mapping (for debugging and reflection)
    names: {
      HttpMethod: "HttpMethod",
      PageContext: "PageContext",
      RedirectResult: "RedirectResult",
      UploadedFile: "UploadedFile",
      ResponseHelper: "ResponseHelper",
      Logger: "Logger",
      LogRotationConfig: "LogRotationConfig",
      LoggerConfig: "LoggerConfig",
      FileManagerConfig: "FileManagerConfig",
      MySQLClient: "MySQLClient",
      MySQLConfig: "MySQLConfig",
      MySQLFactory: "MySQLFactory",
      RedisClient: "RedisClient",
      RedisConfig: "RedisConfig",
      RedisFactory: "RedisFactory",
      LdapClient: "LdapClient",
      LdapConfig: "LdapConfig",
      LdapEntry: "LdapEntry",
      ExcelJSFactory: "ExcelJSFactory",
      TestResult: "TestResult",
      TestSuiteResult: "TestSuiteResult",
      MockPageContext: "MockPageContext",
      MockResponse: "MockResponse",
      MockFetchOptions: "MockFetchOptions",
      TestHelper: "TestHelper",
      PageFunction: "PageFunction",
      Page: "Page",
      AppDeps: "AppDeps",
    } as const,

    // Type constructors (placeholders, handled by TypeScript type system at actual use)
    types: {} as {
      HttpMethod: HttpMethod;
      PageContext: PageContext;
      RedirectResult: RedirectResult;
      UploadedFile: UploadedFile;
      ResponseHelper: ResponseHelper;
      Logger: Logger;
      LogRotationConfig: LogRotationConfig;
      LoggerConfig: LoggerConfig;
      FileManagerConfig: FileManagerConfig;
      MySQLClient: MySQLClient;
      MySQLConfig: MySQLConfig;
      MySQLFactory: MySQLFactory;
      RedisClient: RedisClient;
      RedisConfig: RedisConfig;
      RedisFactory: RedisFactory;
      LdapClient: LdapClient;
      LdapConfig: LdapConfig;
      LdapEntry: LdapEntry;
      ExcelJSFactory: ExcelJSFactory;
      TestResult: TestResult;
      TestSuiteResult: TestSuiteResult;
      MockPageContext: MockPageContext;
      MockResponse: MockResponse;
      MockFetchOptions: MockFetchOptions;
      TestHelper: TestHelper;
      PageFunction: PageFunction<unknown>;
      Page: Page<unknown>;
      AppDeps: AppDeps;
    },
  } as const;
};

export type TypesRegistry = ReturnType<typeof createTypesRegistry>;
