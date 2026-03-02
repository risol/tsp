# TSP Feature Documentation

This document introduces the various features and usage methods of TSP.

## Feature List

### Core Features

- [Dependency Injection](./injection.md) - Type-safe dependency injection
  - Use `Page` wrapper to inject dependencies
  - Complete TypeScript type support
  - Proxy lazy loading mechanism
  - Globally available, no import needed

- [AppDeps Guide](./appdeps.md) - Complete dependency injection guide
  - Session and Cookies usage
  - Custom dependency examples (database, cache, API client, etc.)
  - Best practices and common scenarios

- [Session Management](./session.md) - User session management
  - HMAC-SHA256 signature protection
  - User login/logout
  - Session data storage
  - Automatic expiration and cleanup

- [Cookie Management](./cookies.md) - HTTP Cookie management
  - Set and read Cookies
  - Security options (httpOnly, secure, sameSite)
  - Batch operations
  - URL encoding handling

- [Static Files](./static-files.md) - Static resource serving
  - Automatic MIME type detection
  - ETag and cache support
  - Configurable file extension whitelist

- [Redirect Feature](./redirect.md) - HTTP redirect support
  - Return redirect object to trigger HTTP redirect
  - Support multiple redirect status codes (301, 302, 303, 307, 308)
  - Concise API design

- [Custom Response](./custom-response.md) - Full control over HTTP responses
  - Return custom Response object
  - Support custom status codes, headers, content types
  - Suitable for APIs and special scenarios

- [Error Handling](./error-handling.md) - Complete error handling mechanism
  - Development mode: Show detailed error info and stack trace
  - Production mode: Hide error details, protect sensitive info
  - Support try-catch to catch page errors

### Database Integration

- [MySQL Schema-first API](./mysql.md) - Type-safe MySQL database access
  - Zod schema runtime validation
  - Multiple query modes (multi-row, single row, optional single row, scalar, paginated)
  - Transaction support (auto-commit/rollback)
  - Complete unit tests and E2E tests
  - Interactive demo page: `/mysql-schema-first.tsx`

## Related Documentation

- [Architecture](../architecture.md) - Learn how these features are implemented
- [Development Guide](../development.md) - How to develop new features
- [Testing Overview](../testing/overview.md) - How to test these features

## Usage Suggestions

1. **Dependency Injection**: Suitable for scenarios where helper functions and services need to be shared across multiple pages
2. **Redirect**: Suitable for scenarios like page redirect after form submission, login verification, etc.
3. **Custom Response**: Suitable for building RESTful APIs
4. **Error Handling**: Be sure to disable development mode in production to protect sensitive information

---

[← Back to Documentation Center](../README.md)
