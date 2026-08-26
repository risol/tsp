# Changelog

All notable changes to TSP will be documented in this file.

## [Unreleased]

### Added
- v2.4 Master + IPC embedded Bun Workers with cross-platform lifecycle,
  timeout, crash replacement, backpressure, and hot-reload coverage.
- Native v2 build, package, benchmark, CI, release, and smoke-test workflows.
- v2 route fixtures covering methods, dynamic parameters, cookies, sessions,
  request bodies, cancellation, static assets, and response handling.
- v2-only root workflow. The former TypeScript/Bun application host and its
  compatibility surface are no longer shipped.

## [0.1.0] - 2026-03-02

### Added
- Initial release
- TSP (TypeScript Server Page) template server using Deno + TSX + React
- Direct `.tsp` file execution (like PHP)
- Intelligent module caching with hot reload support
- Type-safe dependency injection system
- Built-in file manager with password protection
- MySQL Schema-first API with Zod validation
- Redis client support
- LDAP client support
- ExcelJS integration for Excel file operations
- Session management
- Cookie management
- Static file serving with caching
- Configuration auto-reload

### Features
- `.tsp` file suffix as route files
- Global type declarations (no imports needed)
- Schema-first data validation
- Hot reload via Deno's watch mode
- Cross-platform compilation
