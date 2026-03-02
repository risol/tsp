# TSP Hot Reload Documentation

## Overview

TSP supports intelligent Hot Reload functionality that automatically detects file changes and recompiles affected modules in development mode without restarting the server.

## How It Works

### Cache Invalidation Mechanism

TSP uses the following strategies to detect file changes:

1. **Main File Modification Time Check**: Check the modification time (mtime) of the TSX file on each request
2. **Dependency File Modification Time Check**: Recursively check all local dependencies (TSX/TS) modification times
3. **Automatic Recompilation**: If any file is modified, automatically recompile affected main files and dependencies

### Dependency Tracking

The system automatically analyzes local imports for each TSX file:

```typescript
// Automatically track these dependencies
import { Component } from "./components/Component";  // TSX component
import { helper } from "../utils/helper";           // TS utility function
import { config } from "./config";                  // JS configuration
```

### Cache Flow

```
Request → getPage() → Check mtime → Cache valid?
                          ↓ YES          ↓ NO
                    Use memory cache    Check dependency mtime
                                        ↓
                                   Dependency modified?
                                        ↓ YES           ↓ NO
                              Recompile dependency chain    Compile main file only
```

## Usage Scenarios

### Development Mode

```bash
# Start server with hot reload (recommended)
./tsp.sh dev

# Or production mode
./tsp.sh start
```

### Binary Mode

Compiled binaries **fully support hot reload**:

```bash
# Compile binary
./tsp.sh build:tspserver:rel

# Run binary (hot reload always enabled)
./dist/release/windows-x64/tspserver
```

**Important Notes**:
- ✅ **Hot reload is default behavior**: All modes (source, binary, production) automatically support hot reload
- ✅ Supports hot reload of arbitrarily nested dependencies
- ✅ Supports automatic recompilation of TSX and TS files
- ✅ Smart cache invalidation based on file modification time (mtime)

### What the `--dev` Parameter Does

The `--dev` parameter **does not affect hot reload functionality**. Its purpose is:

```bash
# Development mode (skip precompilation, show detailed errors)
./tspserver --root ./www --port 9000 --dev
```

- ⏭️ **Skip precompilation at startup**: Compile files on first request (faster startup in dev)
- 🐛 **Show detailed error stack**: Error pages include complete stack traces
- 🚫 **Disable browser caching**: Set `Cache-Control: no-cache` header

## Hot Reload Scenarios

### Scenario 1: Main File Modified

Modify `www/index.tsx`:

```typescript
// Before modification
export default async function() {
  return <h1>Version 1</h1>;
}

// After modification
export default async function() {
  return <h1>Version 2</h1>;
}
```

**Result**: ✅ Automatically recompiles `index.tsp`, refresh page to take effect immediately

### Scenario 2: Component File Modified

Modify `www/components/Header.tsx`:

```typescript
// Before modification
export function Header() {
  return <header>Old Header</header>;
}

// After modification
export function Header() {
  return <header>New Header</header>;
}
```

**Result**: ✅ Automatically recompiles `Header.tsx` and all files that import it (like `index.tsx`)

### Scenario 3: Utility Function Modified

Modify `www/utils/format.ts`:

```typescript
// Before modification
export function formatDate(date: Date) {
  return date.toISOString();
}

// After modification
export function formatDate(date: Date) {
  return date.toLocaleDateString();
}
```

**Result**: ✅ Automatically recompiles `format.ts` and all components that use it

### Scenario 4: Cascade Dependency Modified

```
index.tsx → components/Layout.tsx → components/Navigation.tsx
```

Modify `Navigation.tsx`:

**Result**: ✅ Automatically recompiles the entire dependency chain:
- `Navigation.tsx`
- `Layout.tsx` (depends on Navigation)
- `index.tsx` (depends on Layout)

## Log Output

### Cache Hit

```
[CACHE HIT] www/index.tsx (mtime: 1706534400000)
```

Indicates the file was not modified, using memory cache.

### Cache Miss

```
[CACHE MISS] www/index.tsp - recompiling...
[INFO] Dependencies: D:\GitHub\tsp\www\components\Layout.tsx
[COMPILED] D:\GitHub\tsp\www\index.tsp -> .cache\tsp\index.js
```

Indicates the file was modified and is being recompiled.

### Dependency File Modified

```
[INFO] Dependency modified: D:\GitHub\tsp\www\components\Layout.tsx
[CACHE MISS] www/index.tsp - recompiling...
[INFO] Compiling dependency: D:\GitHub\tsp\www\components\Layout.tsx
```

Indicates a dependency file was modified, recompiling the dependency chain.

## Testing

Run hot reload tests:

```bash
./tsp.sh test
```

Test coverage:

1. ✅ **Main file hot reload**: Modify main TSX file, verify auto reload
2. ✅ **Component hot reload**: Modify dependent TSX component, verify main file auto reload
3. ✅ **Utility file hot reload**: Modify dependent TS utility file, verify main file auto reload
4. ✅ **Cascade hot reload**: Modify bottom file of multi-level dependencies, verify entire chain auto reload

## Performance Considerations

### Development Mode

- **Startup speed**: ⚡ Fast (skip precompilation)
- **First request**: On-demand compilation (slightly slower)
- **Subsequent requests**: Use cache (fast)
- **File changes**: Automatic recompilation (transparent)

### Production Mode

- **Startup speed**: ⏳ Slower (precompile all files)
- **First request**: Use cache (fast)
- **Subsequent requests**: Use cache (fast)
- **File changes**: Automatic recompilation (transparent)

### Memory Usage

- **Module cache**: Only cache compiled functions (lightweight)
- **Dependency graph**: Store file path relationships (small memory footprint)
- **mtime tracking**: Only store timestamp numbers (extremely small)

## Limitations

### Untracked Changes

1. **Remote dependencies**: HTTP/HTTPS/npm/jsr imports not checked for changes
2. **Configuration files**: Modifying `config.json` requires server restart
3. **Outside source code**: Static resource changes (CSS, images, etc.) do not trigger reload

### Security Restrictions

- **Remote imports prohibited**: Check and block remote file imports during compilation
- **Path traversal protection**: Automatically block malicious path access
- **File type whitelist**: Only process `.tsx`, `.ts`, `.js` files

## Best Practices

### 1. Development Workflow

```bash
# Terminal 1: Start dev server
./tsp.sh dev

# Terminal 2: Run tests (optional)
./tsp.sh test
```

### 2. File Organization

Recommended file organization structure:

```
www/
├── pages/           # Page files
│   ├── index.tsx
│   └── about.tsx
├── components/      # Reusable components
│   ├── Header.tsx
│   └── Footer.tsx
└── utils/           # Utility functions
    ├── format.ts
    └── validate.ts
```

### 3. Performance Optimization

- **Avoid excessive imports**: Only import necessary files
- **Share components**: Extract common functionality into separate components
- **Utility functions**: Move complex logic to utility files

## Troubleshooting

### Problem: Modified files don't take effect

**Solutions**:
1. Check if in development mode (`--dev` flag)
2. Check server logs to confirm no compilation errors
3. Confirm file was saved successfully (check file modification time)

### Problem: Dependency file modifications don't trigger reload

**Solutions**:
1. Confirm using local imports (`./` or `../`)
2. Check if import paths are correct
3. Check server logs to confirm dependencies are correctly identified

### Problem: Cache misses causing frequent recompilation

**Solutions**:
1. This is normal behavior, the system handles it automatically
2. Frequent recompilation may be due to file system monitoring issues
3. Use production mode (precompilation) to avoid compilation at startup

## Related Documentation

- [Build and Deployment Documentation](./BUILD.md)
