# 编译问题完整解决方案

## 🔴 问题

访问 `http://127.0.0.1:9000/form.tsx` 时报错：
```
500 Internal Server Error
Unexpected token '{' at file:///D:/GitHub/tsp/www/form.tsx:1:13
```

## 🎯 根本原因

**编译后的可执行文件包含的是旧版本的源代码**

- `deno run` ✅ - 每次运行都读取最新源代码
- `deno compile` ❌ - 编译时嵌入源代码，编译后源代码修改不会反映到可执行文件

## ✅ 解决步骤

### 方法 1：使用 PowerShell 脚本（推荐）

```powershell
# 运行编译脚本
.\test_compile.ps1

# 然后运行服务器
./tsp-fpm.exe -r ./www -p 9000 --dev
```

### 方法 2：手动执行

```powershell
# 1. 删除旧的可执行文件
Remove-Item tsp-fpm.exe -Force

# 2. 验证源代码
deno check www/form.tsx

# 3. 重新编译
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts

# 4. 运行
./tsp-fpm.exe -r ./www -p 9000 --dev
```

### 方法 3：使用 Bash（Git Bash / WSL）

```bash
# 运行编译脚本
bash test_compile.sh

# 然后运行服务器
./tsp-fpm.exe -r ./www -p 9000 --dev
```

## 📋 验证步骤

### 1. 确认源代码已修复

```powershell
# 检查 form.tsx 第 68-72 行
(Get-Content www\form.tsx | Select-Object -Skip 67 | Select-Object -First 5) -join "`n"
```

应该看到：
```
    line-height: 1.5;
  }
  `;

  return (
```

### 2. 验证可执行文件时间戳

```powershell
ls -lh tsp-fpm.exe
```

确保时间戳是最新的（比 www/form.tsx 新）

### 3. 测试编译

```powershell
# 快速语法检查
deno check www/**/*.tsx

# 如果通过，再编译
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts
```

## ⚠️ 重要提示

### 开发 vs 生产

**开发阶段**（频繁修改代码）：
```powershell
# 使用 deno run，实时看到修改
deno run --allow-net --allow-read src/main.ts -r ./www -p 9000 --dev
```

**生产部署**（代码已稳定）：
```powershell
# 先检查所有代码
deno check www/**/*.tsx

# 然后编译
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts

# 分发可执行文件
./tsp-fpm.exe -r ./www -p 9000
```

### 工作流程建议

```powershell
# 1. 开发阶段
> deno run --allow-net --allow-read src/main.ts --dev

# 2. 测试修改
# 刷新浏览器查看效果

# 3. 代码稳定后
> Remove-Item tsp-fpm.exe -Force
> deno compile --allow-net --allow-read --output tsp-fpm src/main.ts

# 4. 部署
> ./tsp-fpm.exe -r ./www -p 9000
```

## 🔍 故障排查

### 问题：编译后还是报错

**原因**：可执行文件缓存了旧代码

**解决**：
```powershell
# 1. 确保删除旧文件
Remove-Item tsp-fpm.exe -Force

# 2. 清除 Deno 缓存
$env:DENO_DIR=""  # PowerShell
# 或
set DENO_DIR=     # CMD

# 3. 重新编译
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts
```

### 问题：编译成功但访问报错

**检查**：
```powershell
# 1. 验证源文件
deno check www/form.tsx

# 2. 检查时间戳
ls -lh tsp-fpm.exe www/form.tsx

# 3. tsp-fpm.exe 的时间戳应该比 www/form.tsx 新
```

### 问题：修改代码后没有效果

**原因**：还在使用旧的可执行文件

**解决**：
```powershell
# 重新编译
Remove-Item tsp-fpm.exe -Force
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts
```

## 📝 快速参考

| 命令 | 用途 | 何时使用 |
|------|------|---------|
| `deno run --allow-net --allow-read src/main.ts` | 直接运行 | 开发阶段 |
| `deno check www/**/*.tsx` | 检查语法 | 编译前验证 |
| `deno compile --allow-net --allow-read --output tsp-fpm src/main.ts` | 编译可执行文件 | 生产部署 |
| `Remove-Item tsp-fpm.exe -Force` | 删除旧文件 | 重新编译前 |

## ✨ 最佳实践

1. **开发时使用 `deno run`**
   - 实时看到代码修改
   - 快速迭代

2. **代码稳定后再编译**
   - 先运行 `deno check` 检查
   - 确认没有语法错误

3. **每次修改代码后重新编译**
   - 删除旧的可执行文件
   - 重新编译

4. **使用编译脚本**
   - `.\test_compile.ps1` (Windows)
   - `bash test_compile.sh` (Linux/Mac)

## 🎯 总结

问题的核心是：**编译后的可执行文件嵌入了源代码，不会自动更新**

解决方法很简单：
1. ✅ 删除旧的可执行文件
2. ✅ 重新编译
3. ✅ 使用新的可执行文件

---

**最后更新**: 2026-01-27 15:45
**状态**: ✅ 已修复
**可执行文件**: tsp-fpm.exe (90MB)
