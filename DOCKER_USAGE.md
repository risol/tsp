# TSP Server Docker 使用指南

## 目录

1. [快速开始](#快速开始)
2. [使用配置文件](#使用配置文件)
3. [挂载目录](#挂载目录)
4. [Docker Compose 配置](#docker-compose-配置)
5. [高级用法](#高级用法)
6. [生产环境部署](#生产环境部署)

---

## 快速开始

### 1. 构建 Docker 镜像

```bash
# Linux/Mac
./docker-build.sh

# Windows
docker-build.bat
```

### 2. 基本运行

```bash
# 最简单的运行方式（使用默认配置）
docker run -d --name tspserver -p 9000:9000 tspserver:latest
```

---

## 使用配置文件

### 配置文件优先级

配置优先级从高到低：

1. **CLI 参数**（容器启动命令中的参数）
2. **环境变量**（`TSP_PORT`, `TSP_ROOT` 等）
3. **配置文件**（`config.json` 或 `config.jsonc`）
4. **默认值**（port=9000, root=/app/www）

### 方式一：挂载配置文件（推荐）

```bash
docker run -d \
  --name tspserver \
  -p 9000:9000 \
  -v $(pwd)/config.jsonc:/app/config.jsonc:ro \
  tspserver:latest
```

**说明**：
- `config.jsonc` 支持 JSONC 格式（可以添加注释）
- `config.json` 是标准 JSON 格式
- `:ro` 表示只读挂载（安全）
- 配置文件会自动被发现和使用

### 方式二：通过环境变量

```bash
docker run -d \
  --name tspserver \
  -p 9000:9000 \
  -e TSP_PORT=8080 \
  -e TSP_ROOT=/app/www \
  tspserver:latest
```

**支持的环境变量**：
- `TSP_PORT`: 服务器端口（默认 9000）
- `TSP_ROOT`: 网站根目录（默认 /app/www）
- `TSP_MODE`: 运行模式（Production/Development）

### 方式三：启动参数（优先级最高）

```bash
docker run -d \
  --name tspserver \
  -p 9000:9000 \
  tspserver:latest \
  -- --root /app/www --port 8080
```

**注意**：参数前需要加 `--`，用于分隔 Docker 参数和应用程序参数。

---

## 挂载目录

### 1. 挂载网站目录（开发推荐）

```bash
docker run -d \
  --name tspserver \
  -p 9000:9000 \
  -v $(pwd)/www:/app/www:ro \
  tspserver:latest
```

**优点**：
- 在宿主机修改文件，容器内立即生效
- 无需重新构建镜像
- 适合开发和调试

**注意事项**：
- 使用 `:ro` 只读挂载更安全
- 如果需要热重载，确保配置文件中 `"dev": true`

### 2. 挂载日志目录

```bash
docker run -d \
  --name tspserver \
  -p 9000:9000 \
  -v $(pwd)/www:/app/www:ro \
  -v $(pwd)/logs:/app/logs \
  tspserver:latest
```

**配置文件示例**：
```json
{
  "accessLogPath": "./logs/access.log",
  "logger": {
    "file": "./logs/server.log"
  }
}
```

### 3. 挂载数据目录

```bash
docker run -d \
  --name tspserver \
  -p 9000:9000 \
  -v $(pwd)/www:/app/www:ro \
  -v $(pwd)/data:/app/data \
  tspserver:latest
```

### 4. 挂载多个目录（完整示例）

```bash
docker run -d \
  --name tspserver \
  -p 9000:9000 \
  -v $(pwd)/config.jsonc:/app/config.jsonc:ro \
  -v $(pwd)/www:/app/www:ro \
  -v $(pwd)/logs:/app/logs \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/uploads:/app/uploads \
  tspserver:latest
```

---

## Docker Compose 配置

项目提供了两个 Docker Compose 配置文件：

### 1. `docker-compose.server.yml`（仅 TSP Server）

用于单独运行 TSP Server，不包含数据库等服务。

```bash
# 启动
docker-compose -f docker-compose.server.yml up -d

# 查看日志
docker-compose -f docker-compose.server.yml logs -f

# 停止
docker-compose -f docker-compose.server.yml down
```

### 2. `docker-compose.yml`（完整服务栈）

包含 MySQL、Redis、LDAP 等开发服务。

```bash
# 启动所有服务
docker-compose up -d

# 只启动 TSP Server
docker-compose up -d tspserver

# 查看服务状态
docker-compose ps

# 停止所有服务
docker-compose down
```

### Docker Compose 配置示例

```yaml
services:
  tspserver:
    image: tspserver:latest
    container_name: tspserver
    restart: unless-stopped

    environment:
      - TSP_PORT=9000
      - TSP_ROOT=/app/www

    ports:
      - "9000:9000"

    volumes:
      # 配置文件
      - ./config.jsonc:/app/config.jsonc:ro

      # 网站目录
      - ./www:/app/www:ro

      # 日志目录
      - ./logs:/app/logs

      # 数据目录
      - ./data:/app/data

    networks:
      - tsp-network

    # 健康检查
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/"]
      interval: 30s
      timeout: 10s
      retries: 3

networks:
  tsp-network:
    driver: bridge
```

---

## 高级用法

### 1. 连接其他容器（MySQL、Redis）

```yaml
services:
  tspserver:
    image: tspserver:latest
    ports:
      - "9000:9000"
    networks:
      - tsp-network
    depends_on:
      - mysql
      - redis

  mysql:
    image: mysql:8.0
    networks:
      - tsp-network

  redis:
    image: redis:7-alpine
    networks:
      - tsp-network

networks:
  tsp-network:
    driver: bridge
```

**在 TSP Server 中连接**：
```typescript
// 使用容器名称作为主机名
const mysqlHost = "mysql";  // 不是 localhost
const redisHost = "redis";  // 不是 localhost
```

### 2. 资源限制

```yaml
services:
  tspserver:
    image: tspserver:latest
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
```

### 3. 自动重启

```yaml
services:
  tspserver:
    image: tspserver:latest
    restart: unless-stopped  # 或 always, on-failure
```

**重启策略**：
- `no`: 不自动重启（默认）
- `always`: 总是重启
- `on-failure`: 只有失败时重启
- `unless-stopped`: 除非手动停止，否则总是重启

### 4. 环境变量文件

创建 `.env` 文件：
```env
TSP_PORT=9000
TSP_ROOT=/app/www
TSP_MODE=Production
```

Docker Compose 引用：
```yaml
services:
  tspserver:
    image: tspserver:latest
    env_file:
      - .env
```

---

## 生产环境部署

### 1. 生产环境配置文件

创建 `config.production.jsonc`：
```jsonc
{
  "root": "./www",
  "port": 9000,
  "dev": false,

  "accessLogPath": "./logs/access.log",

  "logger": {
    "level": "INFO",
    "file": "./logs/server.log",
    "format": "json",  // JSON 格式便于日志分析
    "rotation": {
      "maxSize": 104857600,  // 100MB
      "maxFiles": 30,
      "compress": true,
      "daily": true
    }
  }
}
```

### 2. 生产环境 Docker Compose

```yaml
services:
  tspserver:
    image: tspserver:v1.0.0  # 使用固定版本
    container_name: tspserver-prod
    restart: always

    ports:
      - "9000:9000"

    volumes:
      - ./config.production.jsonc:/app/config.jsonc:ro
      - ./www:/app/www:ro
      - ./logs:/app/logs

    # 资源限制
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 1G
        reservations:
          cpus: '1.0'
          memory: 512M

    # 健康检查
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s

    networks:
      - tsp-network

    # 日志驱动
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

networks:
  tsp-network:
    driver: bridge
```

### 3. 启动生产环境

```bash
# 使用生产配置
docker run -d \
  --name tspserver-prod \
  --restart always \
  -p 9000:9000 \
  -v $(pwd)/config.production.jsonc:/app/config.jsonc:ro \
  -v $(pwd)/www:/app/www:ro \
  -v $(pwd)/logs:/app/logs \
  --cpus="2.0" \
  --memory="1g" \
  tspserver:v1.0.0
```

### 4. 查看日志

```bash
# 容器日志
docker logs -f tspserver

# 应用日志（如果挂载了日志目录）
tail -f ./logs/server.log
tail -f ./logs/access.log
```

### 5. 更新部署

```bash
# 1. 构建新版本
./docker-build.sh v1.0.1

# 2. 停止旧容器
docker stop tspserver
docker rm tspserver

# 3. 启动新容器
docker run -d \
  --name tspserver \
  --restart always \
  -p 9000:9000 \
  -v $(pwd)/config.jsonc:/app/config.jsonc:ro \
  -v $(pwd)/www:/app/www:ro \
  -v $(pwd)/logs:/app/logs \
  tspserver:v1.0.1

# 或使用 docker-compose（零停机）
docker-compose -f docker-compose.server.yml up -d --no-deps --build tspserver
```

---

## 常见问题

### 1. 权限问题

如果遇到日志文件写入权限问题：

```bash
# 在宿主机创建日志目录并设置权限
mkdir -p logs
chmod 777 logs
```

或修改 Dockerfile 以正确设置权限（已包含在当前 Dockerfile 中）。

### 2. 热重载不工作

确保：
- 配置文件中 `"dev": true`
- 挂载目录时使用读写模式（不加 `:ro`）
- 或者使用卷而不是绑定挂载

### 3. 连接数据库失败

- 使用容器名称作为主机名（不是 `localhost`）
- 确保容器在同一网络中
- 检查 `depends_on` 配置

### 4. 性能优化

- 使用生产模式（`"dev": false`）
- 启用日志归档，避免日志文件过大
- 设置合适的资源限制
- 使用多阶段构建减小镜像体积

---

## 示例命令速查

```bash
# 构建
./docker-build.sh v1.0.0

# 运行（最简）
docker run -d -p 9000:9000 tspserver:v1.0.0

# 运行（带配置和挂载）
docker run -d \
  --name tspserver \
  -p 9000:9000 \
  -v $(pwd)/config.jsonc:/app/config.jsonc:ro \
  -v $(pwd)/www:/app/www:ro \
  -v $(pwd)/logs:/app/logs \
  tspserver:v1.0.0

# Docker Compose
docker-compose -f docker-compose.server.yml up -d

# 查看日志
docker logs -f tspserver

# 进入容器
docker exec -it tspserver /bin/bash

# 停止并删除
docker stop tspserver && docker rm tspserver

# 清理
docker system prune -a
```
