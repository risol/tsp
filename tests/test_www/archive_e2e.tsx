/**
 * 文件管理器解压缩功能 E2E 测试
 *
 * 测试场景：
 * 1. 解压 ZIP 文件
 * 2. 压缩单个文件
 * 3. 压缩多个文件
 * 4. 错误处理（不支持格式、权限不足等）
 */

export default async function ArchiveE2ETest() {
  return (
    <div>
      <h1>文件管理器解压缩功能 E2E 测试</h1>
      <p>测试路径：/test-archive</p>

      <h2>测试说明</h2>
      <ul>
        <li>✓ 解压 ZIP 文件到当前目录</li>
        <li>✓ 解压 ZIP 文件到指定目录</li>
        <li>✓ 压缩单个文件为 ZIP</li>
        <li>✓ 压缩多个文件为 ZIP</li>
        <li>✓ 错误处理（不支持格式、文件大小限制等）</li>
      </ul>

      <h2>测试环境</h2>
      <p>此测试需要手动验证以下功能：</p>

      <h3>1. 解压功能测试</h3>
      <ol>
        <li>在文件管理器中上传一个 ZIP 文件</li>
        <li>点击文件旁边的 📦 解压按钮</li>
        <li>在弹出的模态框中确认目标目录</li>
        <li>点击"解压"按钮</li>
        <li>验证文件是否成功解压</li>
      </ol>

      <h3>2. 压缩功能测试</h3>
      <ol>
        <li>在文件管理器中勾选一个或多个文件的复选框</li>
        <li>点击顶部的"📦 压缩选中项"按钮</li>
        <li>在弹出的模态框中输入 ZIP 文件名</li>
        <li>可选：勾选"包含父目录"</li>
        <li>点击"压缩"按钮</li>
        <li>验证 ZIP 文件是否成功创建</li>
      </ol>

      <h3>3. 安全测试</h3>
      <ul>
        <li>尝试解压不支持格式（应被拒绝）</li>
        <li>尝试解压超大文件（超过 1GB 限制）</li>
        <li>尝试压缩超大文件（超过 500MB 限制）</li>
        <li>未登录访问（应返回 401）</li>
      </ul>

      <h2>配置要求</h2>
      <p>确保 config.json 中包含以下配置：</p>
      <pre style={{ background: "#f5f5f5", padding: "10px" }}>
{`{
  "fileManager": {
    "enabled": true,
    "password": "your_password",
    "allowExtract": true,
    "allowCompress": true,
    "allowedArchiveExtensions": ["zip", "tar", "tgz"],
    "maxExtractSize": 1073741824,
    "maxCompressSize": 524288000,
    "maxExtractFileCount": 10000
  }
}`}
      </pre>

      <h2>预期结果</h2>
      <ul>
        <li>✅ 所有支持的压缩格式（ZIP、TAR、TAR.GZ）都能正常解压</li>
        <li>✅ 单个或多个文件都能正常压缩为 ZIP</li>
        <li>✅ 安全限制正常工作（文件大小、格式限制）</li>
        <li>✅ 错误提示友好且准确</li>
        <li>✅ 解压和压缩操作后自动刷新文件列表</li>
      </ul>

      <h2>测试数据准备</h2>
      <p>使用以下命令创建测试用的 ZIP 文件：</p>
      <pre style={{ background: "#f5f5f5", padding: "10px" }}>
{`# Windows
powershell Compress-Archive -Path test.txt -DestinationPath test.zip

# Linux/macOS
zip test.zip test.txt
tar -cvf test.tar test.txt
tar -czvf test.tar.gz test.txt`}
      </pre>
    </div>
  );
}
