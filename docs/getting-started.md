# TSP 快速开始

## 配置文件功能

TSP 现在支持配置文件！让配置更简单。

### 三种使用方式

#### 1. 使用配置文件（推荐）

创建 `config.json`:
```json
{
  "root": "./www",
  "port": 9000,
  "dev": false
}
```

直接运行：
```bash
./tspserver
```

#### 2. 使用 JSONC（支持注释）

创建 `config.jsonc`:
```json
{
  // 文档根目录
  "root": "./www",

  // 监听端口
  "port": 9000,

  // 开发模式
  "dev": false
}
```

#### 3. 命令行参数

```bash
./tspserver --root ./www --port 9000 --dev
```

### 混合使用

命令行参数会覆盖配置文件：

```bash
# 使用配置文件，但临时修改端口
./tspserver --port 8080
```

### 指定配置文件

```bash
./tspserver --config ./my-config.json
```

### 优先级

命令行参数 > 配置文件 > 默认值

## 完整示例

查看详细文档：[配置文件说明](./configuration.md)

## 更多文档

- [命令行参数](./tasks.md) - 完整的命令行参数说明
- [开发指南](./development.md) - 开发环境配置
- [功能特性](./features/README.md) - 了解 TSP 的功能

## 常用命令

```bash
# 开发模式（带自动重载）
deno task dev

# 生产模式
deno task start

# 编译二进制
deno task compile

# 运行测试
deno task test
```
