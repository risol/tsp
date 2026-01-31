# Docker 测试服务

本项目提供 Docker Compose 配置，用于快速启动测试所需的 MySQL 和 Redis 服务。

## 快速开始

### Linux / macOS

```bash
# 启动服务
./docker-start.sh

# 查看日志
docker-compose logs -f

# 停止服务
./docker-stop.sh

# 重启服务
./docker-restart.sh

# 删除所有测试数据
./docker-cleanup.sh
```

### Windows (PowerShell)

```powershell
# 启动服务
.\docker-start.ps1

# 查看日志
docker-compose logs -f

# 停止服务
.\docker-stop.ps1

# 重启服务
.\docker-restart.ps1

# 删除所有测试数据
.\docker-cleanup.ps1
```

## 服务信息

### MySQL 数据库

- **Host**: `127.0.0.1` 或 `localhost`
- **Port**: `3306`
- **Root Password**: `root123456`
- **Database**: `test_db`
- **User**: `test_user`
- **Password**: `test123456`
- **Charset**: `utf8mb4`

### Redis 缓存

- **Host**: `127.0.0.1` 或 `localhost`
- **Port**: `6379`
- **Password**: 无（默认）
- **Database**: 0-15

### 管理工具

- **phpMyAdmin**: http://localhost:8080
  - 用户名: `root`
  - 密码: `root123456`

- **Redis Commander**: http://localhost:8081

## 连接示例

### MySQL 连接

```bash
# 命令行连接
docker-compose exec mysql mysql -uroot -proot123456

# 使用 mysql 客户端
mysql -h 127.0.0.1 -P 3306 -u root -p
# 密码: root123456
```

### Redis 连接

```bash
# 命令行连接
docker-compose exec redis redis-cli

# 使用 redis-cli
redis-cli -h 127.0.0.1 -p 6379
```

## 测试数据

初始化脚本 `docker/mysql/init/01-init.sql` 会创建以下表：

- `users` - 用户表
- `sessions` - 会话表
- `posts` - 文章表
- `configs` - 配置表
- `access_logs` - 访问日志表

并插入测试数据供开发使用。

## 数据持久化

数据存储在 Docker volumes 中：
- `mysql_data` - MySQL 数据文件
- `redis_data` - Redis 数据文件

即使容器被删除，数据也会保留。要删除数据，运行：

```bash
# 完全清理（包括数据）
docker-compose down -v
```

或使用清理脚本：

```bash
# Linux / macOS
./docker-cleanup.sh

# Windows
.\docker-cleanup.ps1
```

清理脚本会：
- ⚠️ 停止所有容器
- ⚠️ 删除所有容器
- ⚠️ 删除所有数据卷（MySQL和Redis的所有数据）
- ✅ 重新启动时会自动重新初始化测试数据

**注意：此操作不可逆，请谨慎使用！**

## 常见命令

```bash
# 查看运行状态
docker-compose ps

# 查看日志（最近 100 行）
docker-compose logs --tail=100 mysql
docker-compose logs --tail=100 redis

# 实时跟踪日志
docker-compose logs -f mysql
docker-compose logs -f redis

# 进入容器
docker-compose exec mysql bash
docker-compose exec redis sh

# 重新创建容器（保留数据）
docker-compose up -d --force-recreate

# 完全清理（删除数据）
docker-compose down -v
```

## 端口说明

| 服务 | 端口 | 用途 |
|------|------|------|
| MySQL | 3306 | 数据库连接 |
| Redis | 6379 | 缓存连接 |
| phpMyAdmin | 8080 | Web 管理界面 |
| Redis Commander | 8081 | Redis 管理 |

## 故障排除

### 端口被占用

如果 3306 或 6379 端口被占用，修改 `docker-compose.yml` 中的端口映射：

```yaml
ports:
  - "3307:3306"  # 改为 3307
```

### 容器无法启动

```bash
# 查看详细日志
docker-compose logs mysql
docker-compose logs redis

# 重新创建容器
docker-compose down
docker-compose up -d --force-recreate
```

### 清理并重新开始

```bash
# 停止并删除所有容器和数据卷
docker-compose down -v

# 删除镜像（可选）
docker rmi mysql:8.0 redis:7-alpine

# 重新启动
./docker-start.sh
```

## 生产环境使用

这些容器仅用于开发和测试。生产环境建议：

1. 修改默认密码
2. 启用 Redis 持久化密码
3. 配置定期备份
4. 限制 root 远程访问
5. 使用专用的数据库服务器
