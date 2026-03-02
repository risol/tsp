# TSP

![Banner](./docs/images/banner.png)

A TypeScript server that executes `.tsp` files directly like PHP, designed for AI-driven development.

## Features

- **Simple to Use** - Execute `.tsp` files directly like PHP
- **Smart Caching** - File modification time-based module caching with excellent performance
- **Hot Reload** - Support for hot reloading with nested dependencies at any depth
- **Secure** - Comprehensive path checking and permission control
- **Full-featured** - Query parameters, POST data, Cookies, redirects, and more
- **Component-based** - Using TSX + React, supporting modern frontend component development
- **Type-safe** - Complete TypeScript type support, Schema-first database API
- **File Manager** - Built-in web file manager with password protection
- **Config Auto-reload** - Configuration changes take effect automatically without restart
- **Static Files** - Support for HTML, CSS, JS, images, and other static files
- **Port Management** - Automatically detect and clean up processes occupying ports
- **Database Integration** - Schema-first MySQL/Redis/LDAP support, type-safe database queries

## Quick Start

### 1. Prepare deno-tsp

Build the project's built-in `deno-tsp` first (no dependency on official Deno executable):

```bash
sh ./tsp.sh build:denort
sh ./tsp.sh build:deno
```

### 2. Start the Server

```bash
# Clone the repository
git clone https://github.com/your-repo/tsp.git
cd tsp

# Start development server
sh ./tsp.sh dev

# Start production mode
sh ./tsp.sh start
```

### 3. Access the Application

Open browser and visit `http://localhost:9000`

## Build Executable

```bash
# Build binary for current platform
sh ./tsp.sh build:tspserver

# Build release binary
sh ./tsp.sh build:tspserver:rel
```

Build output is in the `release/` directory.

## Docker Test Services

The project includes Docker Compose configuration for quickly starting MySQL and Redis services needed for testing.

See [DOCKER_SERVICES.md](docker/DOCKER_SERVICES.md)

## Documentation

- [Getting Started](./docs/getting-started) - Quick start guide
- [Development Guide](./docs/development) - Development setup
- [Configuration](./docs/configuration) - Server configuration
- [Features](./docs/features) - Feature documentation
- [Testing](./docs/testing) - Testing guide
- [Changelog](./docs/changelog) - Version change log
