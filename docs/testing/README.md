# Testing Documentation

## Test Commands

```bash
# All tests
./tsp.sh test

# Unit tests only
./tsp.sh test:unit

# E2E tests only
./tsp.sh test:e2e
```

## Test Structure

```
tests/
├── unit/                    # Unit tests (don't start server)
│   ├── router_test.ts       # Route resolution tests
│   ├── context_test.ts      # Context building tests
│   ├── injection_test.ts    # Dependency injection tests
│   └── security_test.ts     # Security check tests
│
├── test_www/                # E2E test pages
│   ├── index.tsx           # Home page
│   ├── api.tsx             # API test page
│   ├── form.tsx            # Form test page
│   └── ...
│
└── run_*.ts               # Test runners
```

## Test Types

### Unit Tests (`test:unit`)

Test individual module functions without starting server:
- Route resolution
- Context building
- Dependency injection
- Security checks

### E2E Tests (`test:e2e`)

Use compiled binary for end-to-end testing:
- Basic HTTP functionality
- API responses
- Error handling
- Security (path traversal protection)
- Hot reload for nested dependencies

## Test Best Practices

### Development

```bash
./tsp.sh dev    # Development mode (hot reload)
./tsp.sh start  # Production mode
```

### Before Deployment

```bash
./tsp.sh test:e2e  # Ensure binary works correctly
```

## Known Limitations

See [Binary Limitations](../binary-limitations.md):
- JSX Component Import - Not supported in binary
- TS Utility Function Import - Limited in binary

These features work perfectly in source mode (`./tsp.sh dev`).
