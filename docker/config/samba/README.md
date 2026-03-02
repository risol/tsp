# Samba Active Directory Domain Controller 配置

本目录包含 Samba AD DC 容器的配置和初始化脚本。

## 关于 Samba AD DC

**diegogslomp/samba-ad-dc** 是一个完整的 Samba Active Directory Domain Controller 容器镜像，专门为开发和 CI/CD 场景设计。它提供了完整的 AD DC 服务，包括 LDAP、Kerberos、DNS 等功能。

### 特点

- ✅ **轻量级**: 最小化运行时组件，资源占用小
- ✅ **完整 LDAP**: 支持标准 LDAP 协议（端口 389）和 LDAPS（端口 636）
- ✅ **开发友好**: 简化配置，快速启动
- ✅ **多架构支持**: 支持 amd64、arm64、ppc64le 等多种 CPU 架构
- ✅ **标准工具**: 使用标准 LDAP 工具（ldapadd、ldapsearch 等）

## 文件说明

```
config/samba/
├── entrypoint.d/
│   └── create-test-users.sh    # 初始化脚本（自动执行）
├── ad-samba.sh                  # 旧版脚本（已废弃）
└── README.md                    # 本文档
```

### 初始化脚本

**`entrypoint.d/create-test-users.sh`**
- Samba AD 容器会自动执行 `/entrypoint.d` 目录中的所有可执行脚本
- 脚本在 Samba 服务启动后运行
- 使用 samba-tool 工具创建测试用户和组

## 连接信息

### 域控制器信息

```
域名: EXAMPLE.COM
NetBIOS 名: EXAMPLE
Base DN: DC=example,DC=com
管理员: Administrator
密码: P@ssw0rd123
```

### 环境变量

| 变量 | 值 | 说明 |
|------|-----|------|
| `REALM` | `EXAMPLE.COM` | Kerberos 域名（必需，必须在前） |
| `DOMAIN` | `EXAMPLE` | NetBIOS 域名 |
| `ADMIN_PASS` | `P@ssw0rd123` | 管理员密码 |
| `ALLOW_INSECURE_LDAP` | `true` | 允许明文 LDAP（仅开发环境） |
| `DNS_FORWARDER` | `8.8.8.8` | DNS 转发器 |
| `TZ` | `Asia/Shanghai` | 时区 |
| `SERVER_ROLE` | `dc` | 服务器角色 |

### 测试账户

```
用户: testuser1
密码: Test@123
DN: CN=testuser1,OU=Users,DC=example,DC=com

用户: testuser2
密码: Test@123
DN: CN=testuser2,OU=Users,DC=example,DC=com

用户: testuser3
密码: Test@123
DN: CN=testuser3,OU=Users,DC=example,DC=com
```

### 测试组

```
组名: TestGroup
DN: CN=TestGroup,OU=Groups,DC=example,DC=com
成员: testuser1, testuser2
```

## LDAP 连接参数

```typescript
// 连接到 Samba AD
const ldap = await createLdap({
  url: 'ldap://localhost:389',
  bindDN: 'CN=Administrator,CN=Users,DC=example,DC=com',
  bindCredentials: 'P@ssw0rd123',
  baseDN: 'DC=example,DC=com',
  startTLS: false,
  timeout: 5000
});
```

## 常见操作

### 用户认证

```typescript
// 验证用户密码
try {
  await ldap.bind(
    `CN=testuser1,OU=Users,DC=example,DC=com`,
    'Test@123'
  );
  console.log('认证成功');
} catch (error) {
  console.log('认证失败', error);
}
```

### 搜索用户

```typescript
const entries = await ldap.search(
  'OU=Users,DC=example,DC=com',
  {
    scope: 'sub',
    filter: '(objectClass=user)',
    attributes: ['cn', 'mail', 'userPrincipalName'],
    sizeLimit: 100
  }
);
```

### 使用命令行工具

```bash
# 搜索所有用户
docker exec tsp-samba-ad ldapsearch \
  -x -H ldap://localhost:389 \
  -b OU=Users,DC=example,DC=com \
  -D "CN=Administrator,CN=Users,DC=example,DC=com" \
  -w P@ssw0rd123 \
  "(objectClass=user)"

# 验证用户密码
docker exec tsp-samba-ad ldapwhoami \
  -x -H ldap://localhost:389 \
  -D "CN=testuser1,OU=Users,DC=example,DC=com" \
  -w Test@123
```

## 容器配置

### Docker Compose 配置

```yaml
samba-ad:
  image: diegogslomp/samba-ad-dc:latest
  container_name: tsp-samba-ad
  environment:
    - REALM=EXAMPLE.COM
    - DOMAIN=EXAMPLE
    - ADMIN_PASS=P@ssw0rd123
    - DNS_FORWARDER=8.8.8.8
    - ALLOW_INSECURE_LDAP=true
    - TZ=Asia/Shanghai
    - SERVER_ROLE=dc
  ports:
    - "389:389"      # LDAP
    - "636:636"      # LDAPS
    - "3268:3268"    # Global Catalog LDAP
    - "53:53"        # DNS
    - "88:88"        # Kerberos
  volumes:
    - samba_data:/var/lib/samba
    - samba_etc:/etc/samba
    - ./config/samba/entrypoint.d:/entrypoint.d:ro
  privileged: true
  hostname: samba-ad
  healthcheck:
    test: ["CMD-SHELL", "ldapsearch -x -H ldap://localhost:389 -b DC=example,DC=com -D 'CN=Administrator,CN=Users,DC=example,DC=com' -w P@ssw0rd123 '(objectClass=*)' || exit 1"]
    start_period: 60s
```

### 暴露端口

| 端口 | 协议 | 说明 |
|------|------|------|
| 389 | LDAP | 标准 LDAP 连接 |
| 636 | LDAPS | 加密 LDAP 连接 |
| 3268 | GC LDAP | Global Catalog LDAP |
| 53 | DNS | DNS 服务 |
| 88 | Kerberos | Kerberos 认证 |

## 注意事项

1. **首次启动**: 容器首次启动需要约 40-60 秒完成初始化
2. **初始化脚本**: 自定义脚本会在 Samba 服务启动后自动执行
3. **数据持久化**: 建议挂载 `/var/lib/samba` 和 `/etc/samba` 卷以持久化 LDAP 数据
4. **开发环境**: `ALLOW_INSECURE_LDAP` 仅用于开发环境
5. **标准工具**: Samba AD DC 使用 `samba-tool` 进行管理

## 参考资源

- **Docker Hub**: https://hub.docker.com/r/diegogslomp/samba-ad-dc
- **Samba Wiki**: https://wiki.samba.org/

