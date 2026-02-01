# LDAP 服务器配置

使用独立的 Docker 容器运行 OpenLDAP 服务器用于开发和测试。

## 快速启动

### 自动启动（推荐）

```bash
# 启动所有服务（MySQL、Redis、LDAP）
./docker-start.sh        # Linux/Mac
docker-start.bat         # Windows
```

启动脚本会自动：
1. 创建并启动 `docker-test-openldap` 容器
2. 导入测试用户（6个用户）

### 手动启动

```bash
# Linux/Mac
docker run -d --name docker-test-openldap \
  -p 1389:389 \
  -p 1636:636 \
  -e LDAP_ORGANISATION="Example Inc" \
  -e LDAP_DOMAIN="example.org" \
  -e LDAP_ADMIN_PASSWORD="admin123456" \
  osixia/openldap:1.5.0

# Windows
docker run -d --name docker-test-openldap ^
  -p 1389:389 ^
  -p 1636:636 ^
  -e LDAP_ORGANISATION="Example Inc" ^
  -e LDAP_DOMAIN="example.org" ^
  -e LDAP_ADMIN_PASSWORD="admin123456" ^
  osixia/openldap:1.5.0
```

## 服务访问

### OpenLDAP 服务器
- **端口**: 1389 (LDAP), 1636 (LDAPS)
- **连接 URL**: `ldap://localhost:1389`
- **Base DN**: `dc=example,dc=org`
- **管理员 DN**: `cn=admin,dc=example,dc=org`
- **管理员密码**: `admin123456`

## 测试用户

启动脚本会自动导入测试用户（定义在 `test-users.ldif`）：

### 中文姓名用户

| 用户名 | 密码 | DN | 邮箱 |
|--------|------|-----|------|
| 张三 | password123 | `cn=zhang san,ou=developers,dc=example,dc=org` | zhang.san@example.com |
| 李四 | password456 | `cn=li si,ou=developers,dc=example,dc=org` | li.si@example.com |
| 王五 | password789 | `cn=wang wu,ou=developers,dc=example,dc=org` | wang.wu@example.com |

### 英文用户

| 用户名 | 密码 | DN |
|--------|------|-----|
| user01 | password01 | `cn=user01,ou=developers,dc=example,dc=org` |
| user02 | password02 | `cn=user02,ou=developers,dc=example,dc=org` |
| user03 | password03 | `cn=user03,ou=developers,dc=example,dc=org` |

### 手动导入测试用户

如果自动导入失败，可以手动执行：

```bash
# Linux/Mac
./docker/import-ldap-users.sh

# Windows
docker\import-ldap-users.bat
```

## 命令行测试

### 1. 测试连接

```bash
docker exec docker-test-openldap ldapsearch \
  -x -H ldap://localhost:389 \
  -b dc=example,dc=org \
  -D "cn=admin,dc=example,dc=org" \
  -w admin123456 \
  "(objectClass=*)"
```

### 2. 搜索用户

```bash
docker exec docker-test-openldap ldapsearch \
  -x -H ldap://localhost:389 \
  -b ou=developers,dc=example,dc=org \
  -D "cn=admin,dc=example,dc=org" \
  -w admin123456 \
  "(objectClass=person)"
```

### 3. 测试用户认证

```bash
# 测试张三的认证
docker exec docker-test-openldap ldapwhoami \
  -x -H ldap://localhost:389 \
  -D "cn=zhang san,ou=developers,dc=example,dc=org" \
  -w password123
```

## 测试脚本

运行 LDAP 单元测试：

```bash
deno run --allow-all tests/unit/ldap_docker_test.ts
```

## 文件说明

- `test-users.ldif` - 测试用户定义文件
- `custom.ldif` - 自定义 LDAP 数据（旧版，保留用于参考）

## 常用命令

```bash
# 查看容器状态
docker ps | grep docker-test-openldap

# 查看容器日志
docker logs -f docker-test-openldap

# 停止容器
docker stop docker-test-openldap

# 启动容器
docker start docker-test-openldap

# 删除容器
docker rm -f docker-test-openldap
```

## 故障排除

### 容器启动失败

```bash
# 查看错误日志
docker logs docker-test-openldap

# 重新创建容器
docker rm -f docker-test-openldap
./docker-start.sh
```

### 测试用户导入失败

```bash
# 手动导入
./docker/import-ldap-users.sh

# 或者
docker exec -i docker-test-openldap ldapadd \
  -x -H ldap://localhost:389 \
  -D "cn=admin,dc=example,dc=org" \
  -w admin123456 \
  < docker/ldap/test-users.ldif
```

### 端口被占用

如果 1389 或 1636 端口被占用，修改启动命令中的端口映射：

```bash
docker run -d --name docker-test-openldap \
  -p 2389:389 \   # 改用其他端口
  -p 2636:636 \
  ...
```

## 相关文档

- [Docker 服务管理](../../DOCKER_SERVICES.md)
- [LDAP E2E 测试](../../LDAP_E2E_TESTS.md)

```bash
# 使用 ldapsearch 测试（需要安装 ldap-utils）
docker exec -it tsp-openldap ldapsearch \
  -x \
  -H localhost:1389 \
  -b dc=example,dc=org \
  -D "cn=admin,dc=example,dc=org" \
  -w admin123456 \
  "(objectClass=*)"
```

### 2. 测试用户认证

```bash
# 尝试用用户密码绑定
docker exec -it tsp-openldap ldapsearch \
  -x \
  -H localhost:1389 \
  -b dc=example,dc=org \
  -D "cn=zhang san,ou=developers,dc=example,dc=org" \
  -w password123 \
  "(objectClass=*)"
```

### 3. 搜索特定用户

```bash
# 搜索所有用户
docker exec -it tsp-openldap ldapsearch \
  -x \
  -H localhost:1389 \
  -b ou=developers,dc=example,dc=org \
  -D "cn=admin,dc=example,dc=org" \
  -w admin123456 \
  "(objectClass=person)" \
  cn mail uid
```

### 4. 导入自定义 LDIF

```bash
# 导入自定义数据
docker exec -it tsp-openldap ldapadd \
  -x \
  -H localhost:1389 \
  -D "cn=admin,dc=example,dc=org" \
  -w admin123456 \
  -f /ldfiles/custom.ldif
```

### 5. 删除条目

```bash
# 删除用户
docker exec -it tsp-openldap ldapdelete \
  -x \
  -H localhost:1389 \
  -D "cn=admin,dc=example,dc=org" \
  -w admin123456 \
  "cn=wang wu,ou=developers,dc=example,dc=org"
```

## TSP 应用配置

在 TSP 应用中使用此 LDAP 服务器：

```tsx
export default Page(async function(ctx, { createLdap, response }) {
  // 创建 LDAP 客户端连接
  const ldap = await createLdap({
    url: 'ldap://localhost:1389',
    bindDN: 'cn=admin,dc=example,dc=org',
    bindCredentials: 'admin123456',
    baseDN: 'dc=example,dc=org'
  });

  // 搜索用户
  const entries = await ldap.search('ou=developers,dc=example,dc=org', {
    filter: '(objectClass=person)',
    scope: 'sub'
  });

  // 用户认证
  try {
    await ldap.bind(
      'cn=zhang san,ou=developers,dc=example,dc=org',
      'password123'
    );
    return response.json({ success: true, message: '认证成功' });
  } catch (err) {
    return response.json({ success: false, message: '认证失败' });
  } finally {
    await ldap.close();
  }
});
```

## 访问 phpLDAPadmin

1. 打开浏览器访问 http://localhost:8082
2. 登录信息：
   - **Login DN**: `cn=admin,dc=example,dc=org`
   - **Password**: `admin123456`
3. 在左侧树状结构中浏览 LDAP 目录
4. 可以查看、添加、修改、删除条目

## 数据持久化

LDAP 数据存储在 Docker volume `ldap_data` 中，即使删除容器数据也不会丢失。

```bash
# 查看卷
docker volume ls

# 删除卷（会清空所有数据）
docker volume rm tsp_ldap_data
```

## 常见问题

### 端口冲突
如果 1389 端口被占用，修改 `docker-compose.yml`：

```yaml
ports:
  - "2389:1389"  # 改用其他端口
```

### 重置数据

```bash
# 停止服务
docker-compose down

# 删除卷
docker volume rm tsp_ldap_data

# 重新启动
docker-compose up -d
```

### 查看日志

```bash
# 查看日志
docker-compose logs -f openldap

# 查看详细日志
docker exec tsp-openldap cat /opt/bitnami/openldap/logs/openldap.log
```

## 生产环境注意事项

⚠️ **警告**: 此配置仅用于开发和测试环境，不适合生产环境。

生产环境建议：
1. 使用强密码
2. 启用 TLS/SSL (LDAPS)
3. 限制网络访问
4. 定期备份
5. 使用官方 OpenLDAP 镜像并自行配置
