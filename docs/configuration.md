# TSP Configuration File Documentation

## Configuration File Support

TSP supports setting parameters through configuration files while maintaining the flexibility of command line arguments.

## Configuration File Format

### Supported Filenames (by priority)

1. `config.jsonc` - JSON with comments support (recommended)
2. `config.json` - Standard JSON configuration file

### Configuration File Example

```json
{
  "root": "./www",
  "port": 9000,
  "dev": false
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|--------|------|
| `root` | string | `"./www"` | Document root directory |
| `port` | number | `9000` | Listening port |
| `dev` | boolean | `false` | Development mode (show detailed errors, enable hot reload) |
| `accessLogPath` | string | `undefined` | Access log file path (outputs to console if not set) |
| `logger` | object | `{}` | Logger configuration (see details below) |
| `fileManager` | object | `undefined` | File manager configuration (see details below) |

#### Logger Configuration

```jsonc
{
  "logger": {
    "level": "INFO",              // Log level: DEBUG, INFO, WARN, ERROR
    "file": ".logs/app.log",      // Log file path
    "colorize": true,             // Whether to display colored logs in console
    "format": "text"              // Log format: "text" or "json"
  }
}
```

#### File Manager Configuration

```jsonc
{
  "fileManager": {
    "enabled": true,                     // Whether to enable
    "path": "/__filemanager",           // Access path
    "password": "your_password",         // Access password (at least 6 characters)
    "allowOutsideRoot": false,          // Whether to allow access outside root
    "deniedPaths": [".git", ".deno"],   // Paths to deny access
    "maxUploadSize": 104857600          // Maximum upload size (bytes)
  }
}
```

Refer to [File Manager](../features/filemanager.md) for complete file manager configuration.

## Usage

### 1. Use Default Configuration File

Create `config.json` in the project root, then run directly:

```bash
./tspserver
```

TSP will automatically find and load the configuration file.

### 2. Specify Configuration File

```bash
./tspserver --config ./my-config.json
# or
./tspserver -c ./my-config.json
```

### 3. Command Line Argument Override

Command line arguments have higher priority than configuration files:

```bash
# Use config file but override port and dev mode
./tspserver --port 8080 --dev
```

### 4. Pure Command Line Arguments (without config file)

```bash
./tspserver --root ./www --port 9000 --dev
```

### 5. Access Log Configuration

**Console Output (default)**:
```bash
# Don't configure accessLogPath, logs output to console
./tspserver
```

**File Output**:
```bash
# Via command line argument
./tspserver --access-log ./access.log

# Via configuration file
{
  "root": "./www",
  "port": 9000,
  "dev": false,
  "accessLogPath": "./access.log"
}
```

**Log Format**:
```
2026-01-29T10:30:45.123Z GET /index.tsx 200 "Mozilla/5.0..."
```
Format: `timestamp method path statusCode "User-Agent"`

## Configuration Auto-Reload ⭐

**Important Feature**: TSP supports automatic configuration reload - configuration changes take effect without restarting the server.

### Auto-Reload Configuration Items

The following configuration items will take effect automatically after modification:

- ✅ **File Manager Password** - Changes take effect immediately, no restart needed
- ✅ **Logger Configuration** - Log level, file path, etc.
- ✅ **Static File Extensions** - New or modified static file types
- ✅ **File Manager Configuration** - Most file manager settings

The following configuration items require server restart:

- ❌ **Port** - Requires restart because port is bound at startup
- ❌ **Root Directory** - Requires restart because path is resolved at startup

### How It Works

1. Server records configuration file path and modification time at startup
2. Check if configuration file was modified on each request
3. If modification is detected, reload configuration immediately
4. Apply new configuration to subsequent requests

### Usage Examples

**Scenario 1: Change File Manager Password**

```bash
# 1. Edit configuration file, change password
vim config.jsonc

# 2. Refresh browser, login with new password
# No server restart needed!
```

**Scenario 2: Change Log Level**

```bash
# 1. Edit configuration file, change log level
vim config.jsonc
# "logger": { "level": "DEBUG" }

# 2. Configuration takes effect immediately, subsequent requests will log detailed logs
# No server restart needed!
```

### Performance Impact

- Configuration check overhead: ~0.01ms per request (one file stat system call)
- Configuration reload overhead: ~1.5ms (only occurs when file is modified)
- Overall impact: Negligible (configuration changes are infrequent operations)

## Configuration Priority

From high to low:

1. **Command Line Arguments** - Highest priority
2. **Configuration File** - Medium priority
3. **Default Values** - Lowest priority

## Example Scenarios

### Development Environment

**Configuration File** (`config.json`):
```json
{
  "root": "./www",
  "port": 9000,
  "dev": true
}
```

**Startup**:
```bash
./tspserver
```

### Production Environment

**Configuration File** (`config.json`):
```json
{
  "root": "./www",
  "port": 80,
  "dev": false,
  "accessLogPath": "./logs/access.log"
}
```

**Startup**:
```bash
./tspserver
```

### Multi-Environment Configuration

**Development Config** (`config.dev.json`):
```json
{
  "root": "./www",
  "port": 9000,
  "dev": true
}
```

**Production Config** (`config.prod.json`):
```json
{
  "root": "./www",
  "port": 80,
  "dev": false,
  "accessLogPath": "./logs/access.log"
}
```

**Startup**:
```bash
# Development environment
./tspserver --config ./tspserver.dev.json

# Production environment
./tspserver --config ./tspserver.prod.json
```

### Quick Testing

Keep configuration file, temporarily override parameters:

```bash
# Use config file but temporarily switch to different port
./tspserver --port 9999 --dev

# Use config file but temporarily switch to different directory
./tspserver --root ./test-www
```

## Notes

1. **Configuration file must be valid JSON format**
   - Use double quotes `"` instead of single quotes `'`
   - No trailing commas
   - Boolean values use `true`/`false` (lowercase)

2. **JSONC format supports comments**
   - If using `.jsonc` extension, supports `//` and `/* */` comments
   - Standard `.json` does not support comments

3. **Relative Paths**
   - Paths in configuration file are relative to the running directory
   - Recommend using relative paths like `./www`

4. **Error Handling**
   - If configuration file format is incorrect, TSP will show error and exit
   - If specified configuration file does not exist, TSP will show error and exit

## Complete Example

**Project Structure**:
```
my-project/
├── config.json            # Configuration file
├── www/                    # Document root
│   ├── index.tsx
│   └── about.tsx
└── tspserver              # Binary file
```

**Configuration File** (`config.json`):
```json
{
  "root": "./www",
  "port": 3000,
  "dev": true,
  "accessLogPath": "./access.log"
}
```

**Startup**:
```bash
# Use configuration file
./tspserver

# Access http://localhost:3000
# View log file
cat access.log
```

## Troubleshooting

### Configuration File Not Recognized

Check if filename is correct:
- ✅ `config.jsonc`
- ✅ `config.json`
- ❌ `tspserver.config.json`
- ❌ `config.json5`

### Configuration File Format Error

Error example:
```json
{
  "root": './www',    // ❌ Used single quotes
  "port": 9000,       // ❌ Trailing comma (in .json)
}
```

Correct example:
```json
{
  "root": "./www",
  "port": 9000
}
```

### Configuration Changes Not Taking Effect

**Problem**: After modifying configuration file, configuration didn't take effect

**Possible Causes**:
1. Used `.json` extension but included comments
2. JSON format error (syntax error)
3. Configuration item name typo

**Solutions**:
1. Use `.jsonc` extension to support comments
2. Use JSON validator to check format
3. Check server logs to confirm configuration loaded correctly
4. After changing password, refresh browser to use new password (no restart needed)

### Port Already in Use

If configured port is already in use:
```bash
# Temporarily use different port
./tspserver --port 9999
```

## Related Commands

```bash
# View help
./tsp.sh dev --help

# Development mode (auto-reload)
./tsp.sh dev

# Production mode
./tsp.sh start

# Compile binary
./tsp.sh build:tspserver
```

## Related Documentation

- [Getting Started](./getting-started.md) - Quick start in 5 minutes
- [Development Tasks](./tasks.md) - All Deno task descriptions
- [Development Guide](./development.md) - Development environment setup
- [Architecture](./architecture.md) - System architecture description

---

[← Back to Documentation Center](./README.md)
