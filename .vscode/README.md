# VS Code Eta 模板配置指南

本目录包含 VS Code 配置文件，用于识别 `.tsp` 文件为 Eta/EJS 模板。

## 推荐方案 1：使用 EJS 扩展（最简单）

Eta 和 EJS 语法非常相似，可以使用 EJS 扩展：

### 步骤：

1. **安装 EJS 扩展**
   - 打开 VS Code
   - 按 `Ctrl+Shift+X` 打开扩展面板
   - 搜索 `EJS language support` 或 `DigitalBrainstem EJS`
   - 点击安装

2. **验证配置**
   - 打开任意 `.tsp` 文件
   - 点击右下角的语言模式（通常显示为 "HTML"）
   - 选择 "EJS" 或 "JavaScript (EJS)"

## 方案 2：手动配置文件关联

项目已包含以下配置文件：

### `.vscode/settings.json`
```json
{
  "files.associations": {
    "*.tsp": "html",
    "*.jsp": "html"
  }
}
```

这将使 `.tsp` 文件被识别为 HTML，HTML 标签会有高亮。

## 方案 3：使用 Eta 官方扩展

1. 安装 Eta VS Code 扩展（如果存在）
2. 项目已配置 `extensions.json` 推荐安装列表

## 语法高亮测试

打开 `www/index.tsp` 文件，应该看到：

- HTML 标签有颜色高亮
- `<%= %>` 语句应该有特殊颜色
- `<% %>` 代码块应该有特殊颜色

## 如果语法高亮不工作

### 方法 1：强制设置语言模式
1. 打开 `.tsp` 文件
2. 按 `Ctrl+Shift+P`
3. 输入 "Change Language Mode"
4. 选择 "EJS" 或 "HTML"

### 方法 2：使用 EJS 语法
由于 Eta 和 EJS 语法几乎完全兼容，你可以在 VS Code 中：
1. 打开 Settings (Ctrl+,)
2. 搜索 "files.associations"
3. 添加：
   ```json
   "*.tsp": "ejs"
   ```

## 代码片段支持

如果需要代码片段，可以创建 `.vscode/eta.code-snippets`：

```json
{
  "Eta Output": {
    "prefix": "eto",
    "body": "<%= ${1:expression} %>"
  },
  "Eta Code": {
    "prefix": "etc",
    "body": "<%\n  ${1:// code}\n%>"
  }
}
```

## Emmet 支持

配置文件已包含 Emmet 支持，可以在 `.tsp` 文件中使用 Emmet 缩写：

```html
div.container>ul>li*3
```

按 Tab 键会展开为完整的 HTML 结构。

## 故障排除

### 问题：模板标签没有高亮
**解决方案**：安装 EJS 扩展并手动设置语言模式为 EJS

### 问题：JavaScript 代码没有高亮
**解决方案**：这是正常的，HTML 模板中的内嵌 JS 通常不会有完整高亮

### 问题：自动补全不工作
**解决方案**：安装 HTML CSS Support 扩展

## 推荐扩展列表

- **EJS language support** - EJS/Eta 语法高亮
- **HTML CSS Support** - CSS 类名智能提示
- **Auto Rename Tag** - 自动重命名配对标签
- **Bracket Pair Colorizer** - 括号颜色配对
