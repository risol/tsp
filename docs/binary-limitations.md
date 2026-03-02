# Binary Limitations Documentation

## Known Limitations

The following known limitations exist for compiled binaries (using `deno compile`):

### 1. JSX Component Import Issue ⚠️

**Problem Description**: Compiled binaries cannot correctly handle imports of other JSX components in TSX files.

**Example**:
```tsx
// www/index.tsx
import { Header } from "./components/Header.tsx";  // ❌ Will fail in binary
import { Card } from "./components/Card.tsx";      // ❌ Will fail in binary

export default async function(context: PageContext) {
  return (
    <div>
      <Header title="Hello" />
      <Card content="World" />
    </div>
  );
}
```

**Error Message**:
```
[FALLBACK] Using @deno/loader for www\index.tsx
Request error: Import "./components/Layout.tsx" not a dependency and not in import map from
```

**Reason**:
- Binary cannot directly import TSX files
- Need to use `@deno/loader` for transpilation
- Relative path imports in transpiled code cannot be resolved correctly

**Solutions**:
1. **Use in development/source mode**: Run server with `./tsp.sh dev`
2. **Write all JSX in one file**: For simple pages, don't use component imports
3. **Use inline components**: Define and use components in the same file

### 2. TS Utility Function Import Issue ⚠️

**Problem Description**: Compiled binaries also cannot correctly import TS utility functions outside the src directory.

**Example**:
```tsx
// www/utils/helpers.ts
export function formatDate(date: Date): string {
  // ...
}

// www/index.tsx
import { formatDate } from "./utils/helpers.ts";  // ❌ Will fail in binary

export default async function(context: PageContext) {
  const today = formatDate(new Date());
  return <div>{today}</div>;
}
```

**Solutions**:
- Put utility functions in the src directory (if needed in binary)
- Or use `./tsp.sh dev` mode

## Feature Comparison

| Feature | `./tsp.sh dev` Mode | Compiled Binary |
|------|----------------|---------------|
| Basic TSX Pages | ✅ | ✅ |
| Import modules from src/ | ✅ | ✅ |
| Import JSX Components | ✅ | ❌ |
| Import non-src/ TS files | ✅ | ✅ |
| Relative Path Imports | ✅ | ✅ |
| Dependency Injection | ✅ | ✅ |
| Hot Reload | ✅ | ✅ |

### About Hot Reload

**Important Note**: Compiled binaries fully support hot reload functionality without any additional parameters.

- ✅ **Auto-detect file changes**: Check TSX/TS file modification time on each request
- ✅ **Auto-recompile**: Automatically recompile affected modules after file modification
- ✅ **Nested dependencies**: Support hot reload of arbitrarily nested dependency files
- ✅ **Versioned cache**: Use versioned filenames to bypass Deno's import cache

The `--dev` parameter:
- Skips precompilation at startup (compiles on first request)
- Shows detailed error stack information
- Disables browser caching (Cache-Control: no-cache)

## Testing Recommendations

### During Development/Testing

Use source mode for development and testing:

```bash
# Development mode (supports hot reload)
./tsp.sh dev

# Start server
./tsp.sh start
```

### For Deployment

If you need to deploy to production:

1. **Avoid Using JSX Component Imports**
   ```tsx
   // ✅ Recommended - All JSX in one file
   export default async function(context: PageContext) {
     return (
       <div>
         <header>Header Here</header>
         <main>Content Here</main>
       </div>
     );
   }
   ```

2. **Use Verified Patterns**
   - Basic page functionality
   - API routes
   - Redirects
   - Dependency injection

3. **Run Full Tests**
   ```bash
   # Run all tests
   ./tsp.sh test

   # Or run separately
   ./tsp.sh test:unit
   ./tsp.sh test:e2e
   ```

## JSX Import Special Test Suite

We provide a dedicated JSX Import test suite that only runs in source mode:

```bash
./tsp.sh test:e2e
```

**Test Coverage**:
- ✅ JSX component imports (Header.tsx, Card.tsx)
- ✅ TS utility function imports (helpers.ts)
- ✅ Component nested rendering
- ✅ Non-src directory imports
- ✅ Dynamic data passing

**Test Files**:
- `tests/test_jsx_imports.ts` - Test suite
- `tests/test_www/jsx-imports.tsx` - Test page
- `tests/test_www/components/` - JSX components
- `tests/test_www/utils/helpers.ts` - Utility functions

## Technical Details

### Why Does This Problem Occur?

1. **Limitations of Deno compile**
   - Deno compile bundles all dependencies into the binary
   - But dynamic imports of TSX files need transpilation at runtime

2. **@deno/loader fallback**
   - We use `@deno/loader` as a fallback for transpilation
   - But relative path imports in transpiled code cannot be resolved correctly

3. **Missing Complete Module Resolution System**
   - Need to implement a module bundling system similar to webpack
   - Or preprocess all TSX files at compile time

### Possible Future Solutions

1. **Precompile All TSX**
   - Convert all TSX to JS at compile time
   - Include converted files in the bundle

2. **Implement Complete Module System**
   - Similar to Next.js bundling approach
   - Or use Vite's approach

3. **Use Different Bundling Strategies**
   - Explore other bundling tools
   - Or improve existing @deno/loader usage

## Related Documentation

- [Architecture Design](./architecture.md) - System architecture and design principles
- [Testing Overview](./testing/overview.md) - Testing overview and how to run tests
- [Development Guide](./development.md) - Development environment setup and best practices
