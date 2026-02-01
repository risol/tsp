# Docker 服务管理文档

## 📋 概述

TSP 项目使用 Docker Compose 管理两个测试服务：
- **MySQL** - 关系型数据库
- **Redis** - 内存缓存

LDAP 服务使用独立的容器运行，便于独立管理和测试。

## 🚀 快速开始

### Linux/Mac

```bash
# 启动所有服务
./docker-start.sh

# 查看状态和日志
./docker-status.sh

# 测试连接
./docker-test-connection.sh

# 重启服务
./docker-restart.sh

# 停止服务
./docker-stop.sh

# 清理所有数据
./docker-cleanup.sh
```

### Windows

```powershell
# 启动所有服务
docker-start.bat

# 查看状态
docker-compose ps

# 重启服务
docker-restart.bat

# 停止服务
docker-stop.bat
```

## 📊 服务信息

### MySQL 数据库
| 参数 | 值 |
|------|-----|
| 协议 | MySQL 8.0 |
| 主机 | 127.0.0.1 |
| 端口 | 3306 |
| Root 密码 | root123456 |
| 数据库 | test_db |
| 测试用户 | test_user / test123456 |
| 管理工具 | http://localhost:8080 (phpMyAdmin) |

### Redis 缓存
| 参数 | 值 |
|------|-----|
| 镜像 | Redis 7 Alpine |
| 主机 | 127.0.0.1 |
| 端口 | 6379 |
| 密码 | 无（默认） |
| 管理工具 | http://localhost:8081 (Redis Commander) |

### LDAP 目录服务（独立容器）

LDAP 服务使用独立的 Docker 容器运行，便于独立管理和测试。

**启动 LDAP 测试服务器：**

#### Linux/Mac
```bash
# 使用提供的脚本（自动导入测试用户）
./docker/start-ldap.sh

# 或手动启动
docker run -d --name docker-test-openldap \
  -p 1389:389 \
  -p 1636:636 \
  -e LDAP_ORGANISATION="Example Inc" \
  -e LDAP_DOMAIN="example.org" \
  -e LDAP_ADMIN_PASSWORD="admin123456" \
  -e LDAP_TLS_VERIFY_CLIENT="never" \
  osixia/openldap:1.5.0
```

#### Windows
```powershell
# 使用提供的脚本（自动导入测试用户）
docker\start-ldap.bat

# 或手动启动
docker run -d --name docker-test-openldap ^
  -p 1389:389 ^
  -p 1636:636 ^
  -e LDAP_ORGANISATION="Example Inc" ^
  -e LDAP_DOMAIN="example.org" ^
  -e LDAP_ADMIN_PASSWORD="admin123456" ^
  -e LDAP_TLS_VERIFY_CLIENT="never" ^
  osixia/openldap:1.5.0
```

**手动导入测试用户：**

```bash
# Linux/Mac
./docker/import-ldap-users.sh

# Windows
docker\import-ldap-users.bat
```

| 参数 | 值 |
|------|-----|
| 镜像 | osixia/openldap:1.5.0 |
| 容器名 | docker-test-openldap |
| URL | ldap://localhost:1389 |
| LDAPS | ldaps://localhost:1636 |
| Base DN | dc=example,dc=org |
| Admin DN | cn=admin,dc=example,dc=org |
| Admin 密码 | admin123456 |

#### LDAP 测试用户

| DN | 密码 | 邮箱 |
|---|---|---|
| `cn=zhang san,ou=developers,dc=example,dc=org` | password123 | zhang.san@example.com |
| `cn=li si,ou=developers,dc=example,dc=org` | password456 | li.si@example.com |
| `cn=wang wu,ou=developers,dc=example,dc=org` | password789 | wang.wu@example.com |
| `cn=user01,ou=developers,dc=example,dc=org` | password01 | - |
| `cn=user02,ou=developers,dc=example,dc=org` | password02 | - |
| `cn=user03,ou=developers,dc=example,dc=example,dc=org` | password03 | - |

## 🔧 脚本说明

### docker-start.sh / docker-start.bat
启动所有 Docker 服务（MySQL、Redis、LDAP）。

**功能：**
- 检查 Docker 运行状态
- 启动所有服务容器
- 显示服务连接信息
- 显示管理工具访问地址
- 显示常用命令提示

**输出：**
```
╔════════════════════════════════════════╗
║     启动 Docker 测试服务                    ║
╚════════════════════════════════════════╝

✓ 服务启动成功！

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 服务信息：
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🐬 MySQL 数据库:
    Host: 127.0.0.1
    Port: 3306
    ...

🔴 Redis 缓存:
    Host: 127.0.0.1
    Port: 6379
    ...

🔐 LDAP 认证服务:
    URL: ldap://localhost:1389
    Base DN: dc=example,dc=org
    ...
```

### docker-stop.sh / docker-stop.bat
停止所有服务，可选删除数据卷。

**功能：**
- 停止所有容器
- 询问是否删除数据卷
- 清理所有数据（可选）

**交互提示：**
```
🗑️  是否要删除数据卷？（所有数据将丢失）

  警告：此操作将删除以下数据：
  - MySQL 数据库数据
  - Redis 缓存数据
  - LDAP 目录数据
```

### docker-restart.sh / docker-restart.bat
重启所有服务并进行健康检查。

**功能：**
- 重启所有容器
- 显示服务状态
- 执行健康检查（MySQL ping、Redis ping、LDAP search）
- 显示每个服务的运行状态

**输出：**
```
✅ 服务已重启！

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 服务状态：
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Name          Command                        State
tsp-mysql    "docker-entrypoint.sh mysqld"     Up
tsp-redis    "redis-server /usr/local/..."     Up
tsp-openldap "/opt/bitnami/openldap/..."    Up

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🏥 健康检查：
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

MySQL:   ✓ 运行中
Redis:   ✓ 运行中
LDAP:    ✓ 运行中
```

### docker-test-connection.sh
测试所有服务的连接并显示数据信息。

**功能：**
- 测试 MySQL 连接
- 测试 Redis 连接
- 测试 LDAP 连接
- 显示数据库信息
- 显示测试数据
- 显示连接字符串

**测试结果：**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🐬 测试 MySQL 连接...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ MySQL 连接成功
  数据库: test_db
  用户: root, test_user

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔴 测试 Redis 连接...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ Redis 连接成功
  版本: 7.0.0
  数据库数: 0

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔐 测试 LDAP 连接...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ LDAP 连接成功
  Base DN: dc=example,dc=org
  管理员: cn=admin,dc=example,dc=org
  用户数量: 6
```

### docker-status.sh
显示服务状态、资源使用和日志。

**功能：**
- 显示容器状态（CPU、内存、网络、磁盘）
- 显示最近的日志（每个服务最后 5 行）
- 提供实时日志查看命令

**输出示例：**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
💻 资源使用
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🐬 MySQL:
  状态: 运行中
  CPU:    0.50%
  内存:   25.5% / 1.5GiB
  网络:   1.2kB / 5.6kB
  磁盘:   15.3MB / 0B

🔴 Redis:
  状态: 运行中
  ...

🔐 LDAP:
  状态: 运行中
  ...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📝 最近日志（最后20行）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🐬 MySQL 日志:
...

🔴 Redis 日志:
...

🔐 LDAP 日志:
...
```

### docker-cleanup.sh
完全清理所有测试数据。

**⚠️ 警告：** 此操作不可逆！

**功能：**
- 停止所有容器
- 删除所有容器
- 删除所有数据卷
- 清理所有测试数据

**删除的数据：**
- MySQL 数据库（test_db）及所有表和数据
- Redis 缓存中的所有数据
- LDAP 目录服务中的所有数据
- 所有已创建的用户、会话、文章等

## 🔍 故障排除

### 服务无法启动

1. **检查 Docker 是否运行**
   ```bash
   docker info
   ```

2. **查看端口占用**
   ```bash
   # Windows
   netstat -ano | findstr ":3306"
   netstat -ano | findstr ":6379"
   netstat -ano | findstr ":1389"

   # Linux/Mac
   lsof -i :3306
   lsof -i :6379
   lsof -i :1389
   ```

3. **查看容器日志**
   ```bash
   docker-compose logs mysql
   docker-compose logs redis
   docker-compose logs openldap
   ```

### 连接失败

1. **测试连接**
   ```bash
   ./docker-test-connection.sh
   ```

2. **重启服务**
   ```bash
   ./docker-restart.sh
   ```

3. **完全重置**
   ```bash
   ./docker-cleanup.sh
   ./docker-start.sh
   ```

### 性能问题

1. **查看资源使用**
   ```bash
   ./docker-status.sh
   ```

2. **限制资源使用**
   编辑 `docker-compose.yml`，添加资源限制：
   ```yaml
   services:
     mysql:
       deploy:
         resources:
           limits:
             cpus: '1'
             memory: 1G
   ```

## 📁 相关文件

```
.
├── docker-compose.yml           # Docker Compose 配置
├── docker-start.sh             # Linux/Mac 启动脚本
├── docker-start.bat            # Windows 启动脚本
├── docker-stop.sh              # Linux/Mac 停止脚本
├── docker-stop.bat             # Windows 停止脚本
├── docker-restart.sh           # Linux/Mac 重启脚本
├── docker-restart.bat          # Windows 重启脚本
├── docker-test-connection.sh   # 连接测试脚本
├── docker-status.sh            # 状态查看脚本
├── docker-cleanup.sh           # 数据清理脚本
└── docker/
    ├── ldap/
    │   ├── README.md           # LDAP 详细文档
    │   └── custom.ldif         # LDAP 测试数据
    └── mysql/
        └── init/               # MySQL 初始化脚本
```

## 🧪 测试命令

### 连接测试
```bash
# 自动测试所有服务
./docker-test-connection.sh

# 手动测试
docker exec -it tsp-mysql mysql -uroot -proot123456
docker exec -it tsp-redis redis-cli
docker exec -it tsp-openldap ldapsearch -x -H localhost:1389 -b dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=*)"
```

### TSP 应用测试
```bash
# LDAP 连接测试
deno run --allow-all tests/unit/ldap_docker_test.ts

# E2E 测试（包含 LDAP）
deno task test:e2e
```

## 💡 最佳实践

1. **开发前启动服务**
   ```bash
   ./docker-start.sh
   ```

2. **定期检查状态**
   ```bash
   ./docker-status.sh
   ```

3. **测试完成后停止**
   ```bash
   ./docker-stop.sh
   ```

4. **需要时清理数据**
   ```bash
   ./docker-cleanup.sh
   ```

## 🔗 相关文档

- [LDAP 快速开始](LDAP_QUICKSTART.md)
- [LDAP E2E 测试](LDAP_E2E_TESTS.md)
- [Docker LDAP 配置](docker/ldap/README.md)
- [TSP 文档](../README.md)
