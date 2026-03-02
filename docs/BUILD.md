# TSP Build and Deployment Documentation

## Overview

TSP provides a complete build toolchain to package projects into independent distributable versions.

## Prerequisites

### Linux Build Dependencies

If building on Linux, install the required dependencies:

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y build-essential clang pkg-config libsqlite3-dev

# CentOS/RHEL
sudo yum install -y gcc gcc-c++ sqlite-devel

# Alpine
apk add --no-cache build-base sqlite-dev
```

## Build Commands

### Full Build

Use the build command to create a complete release package:

```bash
./tsp.sh build:tspserver
```

This command will:
1. Compile binary to `dist/release/`
2. Copy configuration file to output directory
3. Copy types.d.ts to output directory

### Other Build Commands

```bash
# Build release version (default)
./tsp.sh build:tspserver

# Build debug version
./tsp.sh build:tspserver:dev

# Build release version (alias)
./tsp.sh build:tspserver:rel

# First build deno-tsp (required)
./tsp.sh build:denort
./tsp.sh build:deno
```

## Build Output

After building, the `dist/` directory structure is as follows:

```
dist/
├── tspserver.exe          # Windows binary
│   or tspserver           # Linux/Mac binary
├── www/                   # Website files
│   ├── index.tsx
│   ├── form.tsx
│   ├── api.tsx
│   ├── components/
│   │   ├── Layout.tsx
│   │   ├── Header.tsx
│   │   └── ...
│   └── features/
│       └── ...
├── kill-port.sh           # Port cleanup script (Bash)
├── config.json            # Optional configuration file
├── config.jsonc           # Optional configuration file
└── README.md              # Usage instructions
```

## Port Management Tool

The release package includes a port cleanup script to clear occupied ports before starting the server.

### Bash (Linux/macOS/Windows Git Bash)

```bash
# Clean up port configured in config file
bash ./kill-port.sh

# Or grant execute permission and run directly
chmod +x ./kill-port.sh
./kill-port.sh
```

### Integration with Startup

```bash
# One-click startup
bash ./kill-port.sh && ./tspserver
```

## Deployment Steps

### 1. Build Project

```bash
./tsp.sh build:tspserver:rel
```

### 2. Test Build Output

```bash
# Enter dist directory
cd dist/release/windows-x64

# Start server to test
# Windows
.\tspserver.exe

# Linux/Mac
./tspserver
```

### 3. Package for Distribution

#### Windows

```bash
# Using PowerShell
Compress-Archive -Path dist\* -DestinationPath tspserver-windows.zip

# Or using 7-Zip / WinRAR
```

#### Linux/Mac

```bash
# Create tar.gz archive
tar -czf tspserver-linux.tar.gz -C dist .

# Or create zip
cd dist
zip -r ../tspserver-linux.zip .
cd ..
```

### 4. Deploy to Server

Upload the packaged files to the target server:

```bash
# Using scp to upload
scp tspserver-linux.tar.gz user@server:/path/to/deploy/

# Unzip on server
ssh user@server
cd /path/to/deploy
tar -xzf tspserver-linux.tar.gz
cd dist
DENO_DIR=./.deno ./tspserver
```

## Server Configuration

### Using Configuration File

Create `config.json` or `config.jsonc` in `dist/`:

```json
{
  "root": "./www",
  "port": 9000,
  "dev": false
}
```

### Using Command Line Arguments

```bash
# Specify port
./tspserver --port 8080

# Specify document root
./tspserver --root /var/www

# Development mode
./tspserver --dev

# Combined
./tspserver --root ./www --port 9000 --dev
```

## Environment Variables

### DENO_DIR

Compiled binaries require setting the `DENO_DIR` environment variable:

```bash
# Set DENO_DIR to current directory
export DENO_DIR=./.deno  # Linux/Mac
set DENO_DIR=./.deno     # Windows CMD
$env:DENO_DIR="./.deno"  # Windows PowerShell

# Then run the server
./tspserver
```

### One-time Setup

```bash
# Linux/Mac
DENO_DIR=./.deno ./tspserver

# Windows PowerShell
$env:DENO_DIR="./.deno"; .\tspserver.exe

# Windows CMD
set DENO_DIR=./.deno&& tspserver.exe
```

## Production Deployment Recommendations

### 1. Use Process Manager

#### systemd (Linux)

Create `/etc/systemd/system/tsp.service`:

```ini
[Unit]
Description=TSP Server
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/tsp
Environment="DENO_DIR=./.deno"
ExecStart=/opt/tsp/tspserver --root ./www --port 9000
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable tsp
sudo systemctl start tsp
```

#### PM2

```bash
# Install PM2
npm install -g pm2

# Start service
pm2 start tspserver --name tsp -- --root ./www --port 9000

# Enable startup
pm2 startup
pm2 save
```

### 2. Use Reverse Proxy

#### Nginx

```nginx
server {
    listen 80;
    server_name example.com;

    location / {
        proxy_pass http://localhost:9000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

#### Caddy

```
example.com {
    reverse_proxy localhost:9000
}
```

### 3. Use Docker

Create `Dockerfile`:

```dockerfile
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy build output
COPY dist/ /app/

# Expose port
EXPOSE 9000

# Set environment variable and start
ENV DENO_DIR=./.deno
CMD ["./tspserver", "--root", "./www", "--port", "9000"]
```

Build and run:

```bash
# Build image
docker build -t tsp-server .

# Run container
docker run -d -p 9000:9000 --name tsp tsp-server
```

### 4. Security Recommendations

1. **File Permissions**
   ```bash
   # Set appropriate file permissions
   chmod 750 tspserver
   chmod -R 640 www/
   chmod +X www/  # Set directory execute permission
   ```

2. **Firewall Configuration**
   ```bash
   # UFW (Ubuntu)
   sudo ufw allow 9000/tcp

   # firewalld (CentOS)
   sudo firewall-cmd --add-port=9000/tcp --permanent
   sudo firewall-cmd --reload
   ```

3. **HTTPS Configuration**
   - Use Let's Encrypt to get free SSL certificates
   - Configure Nginx/Caddy for HTTPS
   - Set up automatic certificate renewal

## Clean Build Output

```bash
# Delete only dist directory
rm -rf dist/
```

## FAQ

### Q: Why is the DENO_DIR environment variable needed?

A: Compiled binaries need the Deno runtime to handle module caching and dynamic imports. `DENO_DIR` specifies the cache directory location.

### Q: Can I run it on a machine without Deno installed?

A: The Deno runtime must be installed. The binary contains your application code but depends on the Deno runtime environment.

### Q: How to reduce binary file size?

A: You can use the `--no-remote` option to avoid embedding remote modules:

```bash
deno compile --no-remote --allow-net --allow-read --allow-write --allow-env --output tspserver src/main.ts
```

Note: This requires network connectivity on the target machine to download dependencies.

### Q: How to debug compiled binaries?

A: You can use `--log` level or enable development mode:

```bash
./tspserver --dev
```

## Related Documentation

- [Precompilation Documentation](./PRECOMPILATION.md)
- [Coding Standards](./CODING_STANDARDS.md)
- [README](../README.md)
