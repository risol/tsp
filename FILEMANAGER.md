# TSP 文件管理器

TSP 内置的 Web 文件管理器，提供在线文件浏览、上传、下载、重命名、删除等功能。

## 功能特性

- **安全认证**：基于密码的身份验证机制
- **路径控制**：可配置是否允许访问网站 root 目录以外的路径
- **文件操作**：上传、下载、删除、重命名、创建目录
- **权限控制**：细粒度的操作权限配置
- **安全保护**：路径验证、文件类型限制、大小限制
- **友好界面**：现代化的 Web 界面，支持拖拽上传

## 快速开始

### 1. 启用文件管理器

在配置文件中添加 `fileManager` 配置段：

**config.jsonc**:
```jsonc
{
  "root": "./www",
  "port": 9000,
  "fileManager": {
    "enabled": true,
    "password": "your-secure-password"
  }
}
```

### 2. 访问文件管理器

启动服务器后，在浏览器中打开：

```
http://localhost:9000/__filemanager
```

默认路径是 `/__filemanager`，可以通过配置修改。

### 3. 登录

输入配置的密码即可登录。

## 配置选项

完整的配置选项：

```jsonc
{
  "fileManager": {
    // 是否启用（必需）
    "enabled": true,

    // 访问路径（可选，默认："/__filemanager"）
    "path": "/__filemanager",

    // 访问密码（必需，至少 6 个字符）
    "password": "your-secure-password",

    // 是否允许访问 root 外（可选，默认：false）
    "allowOutsideRoot": false,

    // 允许的路径白名单（可选）
    "allowedPaths": ["./www", "./uploads"],

    // 禁止的路径黑名单（可选，默认：[".git", ".deno", "node_modules", ".cache"]）
    "deniedPaths": [".git", ".deno", "node_modules", ".cache"],

    // 最大上传大小（可选，默认：100MB）
    "maxUploadSize": 104857600,

    // 允许上传的扩展名（可选，空列表表示允许所有）
    "allowedExtensions": [".jpg", ".png", ".gif", ".pdf", ".doc", ".docx"],

    // 禁止上传的扩展名（可选，默认：[".exe", ".sh", ".bat", ".cmd", ".scr", ".pif"]）
    "deniedExtensions": [".exe", ".sh", ".bat"],

    // 是否允许删除（可选，默认：true）
    "allowDelete": true,

    // 是否允许重命名（可选，默认：true）
    "allowRename": true,

    // 是否允许创建目录（可选，默认：true）
    "allowMkdir": true,

    // 是否允许移动（可选，默认：false）
    "allowMove": false
  }
}
```

### 配置说明

#### enabled
- 类型：`boolean`
- 默认值：`false`
- 说明：是否启用文件管理器

#### path
- 类型：`string`
- 默认值：`"/__filemanager"`
- 说明：文件管理器的访问路径
- 注意：必须以 `/` 开头，不能是根路径 `/`

#### password
- 类型：`string`
- 默认值：无
- 说明：访问密码
- 要求：至少 6 个字符

#### allowOutsideRoot
- 类型：`boolean`
- 默认值：`false`
- 说明：是否允许访问网站根目录以外的路径
- 安全建议：保持 `false`，除非你确实需要访问其他目录

#### allowedPaths
- 类型：`string[]`
- 默认值：`[]`
- 说明：允许访问的路径白名单
- 用法：当 `allowOutsideRoot` 为 `true` 时，限制只能访问这些路径

#### deniedPaths
- 类型：`string[]`
- 默认值：`[".git", ".deno", "node_modules", ".cache"]`
- 说明：禁止访问的路径黑名单
- 用法：匹配路径中包含的任何模式都会被拒绝

#### maxUploadSize
- 类型：`number`
- 默认值：`104857600`（100MB）
- 说明：最大上传文件大小（字节）

#### allowedExtensions
- 类型：`string[]`
- 默认值：`[]`
- 说明：允许上传的文件扩展名白名单
- 用法：空列表表示允许所有类型（除非被黑名单拒绝）

#### deniedExtensions
- 类型：`string[]`
- 默认值：`[".exe", ".sh", ".bat", ".cmd", ".scr", ".pif"]`
- 说明：禁止上传的文件扩展名黑名单

#### allowDelete
- 类型：`boolean`
- 默认值：`true`
- 说明：是否允许删除文件和目录

#### allowRename
- 类型：`boolean`
- 默认值：`true`
- 说明：是否允许重命名文件和目录

#### allowMkdir
- 类型：`boolean`
- 默认值：`true`
- 说明：是否允许创建新目录

#### allowMove
- 类型：`boolean`
- 默认值：`false`
- 说明：是否允许移动文件和目录

## 使用界面

### 登录页面

首次访问时会显示登录页面，输入配置的密码即可登录。

### 主界面

登录后进入文件管理器主界面：

- **面包屑导航**：显示当前路径，点击可跳转到上级目录
- **上传按钮**：点击打开上传对话框，支持点击选择或拖拽上传
- **新建目录按钮**：创建新目录
- **退出按钮**：退出登录

### 文件列表

文件列表显示以下信息：

- **图标**：文件类型图标
- **名称**：文件名，目录可点击进入
- **大小**：文件大小
- **修改时间**：最后修改时间
- **操作**：
  - 📥 下载
  - ✏️ 重命名
  - 🗑️ 删除

## API 端点

文件管理器提供以下 API 端点（所有 API 都需要认证）：

### POST `/__filemanager/api/login`
登录

**请求体**:
```json
{
  "password": "your-password"
}
```

**响应**:
```json
{
  "success": true,
  "data": {
    "csrfToken": "csrf-token"
  }
}
```

### POST `/__filemanager/api/logout`
登出

**响应**:
```json
{
  "success": true
}
```

### GET `/__filemanager/api/browse?path=/path/to/dir`
浏览目录

**响应**:
```json
{
  "success": true,
  "data": {
    "path": "/path/to/dir",
    "parentPath": "/path/to",
    "files": [
      {
        "name": "example.txt",
        "isDirectory": false,
        "size": 1024,
        "modifiedTime": "2024-01-15T10:30:00.000Z",
        "extension": ".txt"
      }
    ]
  }
}
```

### POST `/__filemanager/api/upload?path=/path/to/dir`
上传文件

**请求**: `multipart/form-data`
- `file`: 文件

**响应**:
```json
{
  "success": true,
  "data": {
    "message": "文件上传成功"
  }
}
```

### GET `/__filemanager/api/download?path=/path/to/file`
下载文件

**响应**: 文件内容

### POST `/__filemanager/api/delete`
删除文件或目录

**请求体**:
```json
{
  "path": "/path/to/file"
}
```

**响应**:
```json
{
  "success": true,
  "data": {
    "message": "删除成功"
  }
}
```

### POST `/__filemanager/api/rename`
重命名文件或目录

**请求体**:
```json
{
  "oldPath": "/path/to/oldname",
  "newName": "newname"
}
```

**响应**:
```json
{
  "success": true,
  "data": {
    "message": "重命名成功"
  }
}
```

### POST `/__filemanager/api/mkdir`
创建目录

**请求体**:
```json
{
  "parentPath": "/path/to/parent",
  "dirName": "newdir"
}
```

**响应**:
```json
{
  "success": true,
  "data": {
    "message": "目录创建成功"
  }
}
```

## 安全建议

### 1. 使用强密码

密码至少 6 个字符，建议使用更长、更复杂的密码：

```jsonc
{
  "fileManager": {
    "password": "MyV3ryS3cur3P@ssw0rd!2024"
  }
}
```

### 2. 限制路径访问

默认情况下，文件管理器只能访问网站根目录内的文件。不要启用 `allowOutsideRoot`，除非确实需要。

```jsonc
{
  "fileManager": {
    "allowOutsideRoot": false
  }
}
```

### 3. 配置路径黑名单

确保敏感目录在黑名单中：

```jsonc
{
  "fileManager": {
    "deniedPaths": [
      ".git",
      ".deno",
      "node_modules",
      ".cache",
      ".env",
      "config"
    ]
  }
}
```

### 4. 限制文件类型

如果只需要上传特定类型的文件，配置白名单：

```jsonc
{
  "fileManager": {
    "allowedExtensions": [".jpg", ".png", ".gif", ".pdf", ".doc", ".docx"]
  }
}
```

### 5. 限制操作权限

如果不需要某些操作，可以禁用：

```jsonc
{
  "fileManager": {
    "allowDelete": false,
    "allowMove": false
  }
}
```

### 6. 使用 HTTPS

生产环境强烈建议使用 HTTPS，保护密码和 session：

```bash
# 使用 Nginx 或 Caddy 反向代理，启用 HTTPS
```

### 7. 定期备份

删除操作不可逆，建议定期备份：

```bash
# 定期备份脚本
tar -czf backup-$(date +%Y%m%d).tar.gz ./www
```

## 故障排查

### 问题：无法访问文件管理器

**可能原因**：
- 文件管理器未启用（`enabled: false`）
- 路径配置错误
- 密码未设置

**解决方法**：
检查配置文件，确保 `fileManager.enabled` 为 `true`，并且设置了密码。

### 问题：登录后显示"未认证"

**可能原因**：
- Cookie 被阻止
- Session 过期

**解决方法**：
确保浏览器允许 Cookie，重新登录。

### 问题：无法上传文件

**可能原因**：
- 文件大小超过限制
- 文件类型被禁止
- 目标目录权限不足

**解决方法**：
检查 `maxUploadSize`、`allowedExtensions`、`deniedExtensions` 配置。

### 问题：无法访问某些目录

**可能原因**：
- 目录在黑名单中
- 超出了 root 目录范围
- 路径白名单限制

**解决方法**：
检查 `deniedPaths`、`allowedPaths`、`allowOutsideRoot` 配置。

## 常见用例

### 用例 1：仅允许上传图片

```jsonc
{
  "fileManager": {
    "enabled": true,
    "password": "image-upload",
    "allowedExtensions": [".jpg", ".jpeg", ".png", ".gif", ".webp"],
    "allowDelete": false,
    "allowRename": false
  }
}
```

### 用例 2：多目录管理

```jsonc
{
  "fileManager": {
    "enabled": true,
    "password": "multi-dirs",
    "allowOutsideRoot": true,
    "allowedPaths": ["./www", "./uploads", "./backups"],
    "deniedPaths": [".git", ".env"]
  }
}
```

### 用例 3：只读模式（只能浏览和下载）

```jsonc
{
  "fileManager": {
    "enabled": true,
    "password": "readonly",
    "allowDelete": false,
    "allowRename": false,
    "allowMkdir": false,
    "allowMove": false
  }
}
```

## 注意事项

1. **路径穿越保护**：文件管理器会自动阻止路径穿越攻击（如 `../../../etc/passwd`）
2. **隐藏文件**：默认不显示以 `.` 开头的隐藏文件
3. **Session 过期**：Session 有效期为 2 小时，超时需要重新登录
4. **并发限制**：当前版本没有并发上传限制，大量文件上传可能影响性能
5. **日志记录**：所有操作都会记录到 TSP 的访问日志中

## 后续优化

计划中的功能改进：

- 分块上传支持大文件
- 文件预览（图片、文本、PDF）
- 批量操作（选择多个文件）
- 搜索功能
- 缩略图生成
- 基于路径的权限配置
- 多用户支持
- 操作审计日志

## 技术实现

文件管理器采用以下技术：

- **后端**：Deno + TypeScript
- **前端**：原生 HTML + CSS + JavaScript（无框架依赖）
- **认证**：PBKDF2 密码哈希 + Session 管理
- **安全**：路径验证、CSRF Token、文件类型检查

## 许可证

TSP 文件管理器遵循与 TSP 相同的许可证。
