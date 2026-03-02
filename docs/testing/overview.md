# TSP Testing Overview

This document introduces TSP's testing framework and testing methods.

## Testing Documentation

### Testing Guides

## Quick Start

### Run All Tests

```bash
./tsp.sh test
```

### Run Unit Tests

```bash
./tsp.sh test:unit
```

### Run E2E Tests

```bash
./tsp.sh test:e2e
```

### Run Specific Test

```bash
./tsp.sh test:unit
```

## Test Structure

```
tests/
├── unit/                    # Unit tests (don't start server)
│   ├── router_test.ts      # Route resolution tests
│   ├── context_test.ts     # Context building tests
│   ├── security_test.ts    # Security check tests
│   └── injection_test.ts   # Dependency injection tests
│
├── e2e/                     # E2E tests (start binary server)
│   └── test_utils.ts       # Shared utility functions
│
├── test_www/                # Test pages
│   ├── index.tsx           # Home page test
│   ├── form.tsx            # Form test
│   ├── api.tsx            # API test
│   └── error.tsx           # Error handling test
│
├── run_unit_tests.ts        # Unit test runner
├── run_e2e_tests.ts         # E2E test runner
└── run_all_tests.ts         # All test runner
```

## Test Configuration

### E2E Test Configuration

- **Test Port**: 9001 (avoid conflict with dev server)
- **Test Directory**: `./tests/test_www`
- **Binary**: Uses `dist/<os>-<arch>-v<version>-dev/tspserver` (must be built first with `./tsp.sh build:tspserver`)

### Unit Test Configuration

- **No server needed** - Directly test module functions
- **Fast execution** - Suitable for quick verification during development
- **Isolated tests** - Each test file runs independently

## Test Coverage

Current test coverage includes:

- ✅ Route resolution and path mapping
- ✅ Context building and type checking
- ✅ Security checks (path traversal, file type)
- ✅ Dependency injection functionality
- ✅ HTTP basic functionality (GET, POST)
- ✅ Form handling
- ✅ Error handling
- ✅ Redirect functionality
- ✅ Custom responses

## Related Documentation

- [Development Guide](../development.md) - How to write tests
- [Task Configuration](../../deno.json) - Deno task descriptions
- [Feature Documentation](../features/README.md) - Tested feature documentation

---

[← Back to Documentation Center](../README.md)
