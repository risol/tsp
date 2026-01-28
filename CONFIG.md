# TSP-FPM 配置文件使用说明

## 配置文件支持

TSP-FPM 支持通过配置文件来设置参数，同时保留命令行参数的灵活性。

## 配置文件格式

### 支持的文件名（按优先级）

1. `config.jsonc` - 支持注释的 JSON（推荐）
2. `config.json` - 标准 JSON 配置文件

### 配置文件示例

```json
{
  "root": "./www",
  "port": 9000,
  "dev": false
}
```

### 配置项说明

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `root` | string | `"./www"` | 文档根目录 |
| `port` | number | `9000` | 监听端口 |
| `dev` | boolean | `false` | 开发模式（显示详细错误） |

## 使用方式

### 1. 使用默认配置文件

在项目根目录创建 `config.json`，然后直接运行：

```bash
./tsp-fpm
```

TSP-FPM 会自动查找并加载配置文件。

### 2. 指定配置文件

```bash
./tsp-fpm --config ./my-config.json
# 或
./tsp-fpm -c ./my-config.json
```

### 3. 命令行参数覆盖

命令行参数的优先级高于配置文件：

```bash
# 使用配置文件，但覆盖端口和开发模式
./tsp-fpm --port 8080 --dev
```

### 4. 纯命令行参数（不使用配置文件）

```bash
./tsp-fpm --root ./www --port 9000 --dev
```

## 配置优先级

从高到低：

1. **命令行参数** - 最高优先级
2. **配置文件** - 中等优先级
3. **默认值** - 最低优先级

## 示例场景

### 开发环境

**配置文件** (`config.json`):
```json
{
  "root": "./www",
  "port": 9000,
  "dev": true
}
```

**启动**:
```bash
./tsp-fpm
```

### 生产环境

**配置文件** (`config.json`):
```json
{
  "root": "./www",
  "port": 80,
  "dev": false
}
```

**启动**:
```bash
./tsp-fpm
```

### 多环境配置

**开发配置** (`config.dev.json`):
```json
{
  "root": "./www",
  "port": 9000,
  "dev": true
}
```

**生产配置** (`config.prod.json`):
```json
{
  "root": "./www",
  "port": 80,
  "dev": false
}
```

**启动**:
```bash
# 开发环境
./tsp-fpm --config ./tsp-fpm.dev.json

# 生产环境
./tsp-fpm --config ./tsp-fpm.prod.json
```

### 快速测试

保留配置文件，临时覆盖参数：

```bash
# 使用配置文件，但临时切换到其他端口
./tsp-fpm --port 9999 --dev

# 使用配置文件，但临时切换到其他目录
./tsp-fpm --root ./test-www
```

## 注意事项

1. **配置文件必须是有效的 JSON 格式**
   - 使用双引号 `"` 而不是单引号 `'`
   - 不能有尾随逗号
   - 布尔值使用 `true`/`false`（小写）

2. **JSONC 格式支持注释**
   - 如果使用 `.jsonc` 扩展名，支持 `//` 和 `/* */` 注释
   - 标准的 `.json` 不支持注释

3. **相对路径**
   - 配置文件中的路径是相对于运行目录的
   - 建议使用 `./www` 这样的相对路径

4. **错误处理**
   - 如果配置文件格式错误，TSP-FPM 会显示错误并退出
   - 如果指定的配置文件不存在，TSP-FPM 会显示错误并退出

## 完整示例

**项目结构**:
```
my-project/
├── config.json            # 配置文件
├── www/                    # 文档根目录
│   ├── index.tsx
│   └── about.tsx
└── tsp-fpm                 # 二进制文件
```

**配置文件** (`config.json`):
```json
{
  "root": "./www",
  "port": 3000,
  "dev": true
}
```

**启动**:
```bash
# 使用配置文件
./tsp-fpm

# 访问 http://localhost:3000
```

## 故障排查

### 配置文件未被识别

检查文件名是否正确：
- ✅ `config.jsonc`
- ✅ `config.json`
- ❌ `tsp-fpm.config.json`
- ❌ `config.json5`

### 配置文件格式错误

错误示例：
```json
{
  "root": './www',    // ❌ 使用了单引号
  "port": 9000,       // ❌ 有尾随逗号（在 .json 中）
}
```

正确示例：
```json
{
  "root": "./www",
  "port": 9000
}
```

### 端口被占用

如果配置的端口已被占用：
```bash
# 临时使用其他端口
./tsp-fpm --port 9999
```

## 相关命令

```bash
# 查看帮助
./tsp-fpm --help

# 开发模式（自动重载）
deno task dev

# 生产模式
deno task start

# 编译二进制文件
deno task compile
```
