export default Page(async function(ctx, { response }) {
  const action = ctx.query.action || "demo";

  // 处理单文件上传
  if (action === "upload-single" && ctx.method === "POST") {
    const file = ctx.files.file as UploadedFile;

    if (!file) {
      return response.error("没有上传文件", 400);
    }

    try {
      // 创建 uploads 目录
      const uploadDir = "./uploads";
      try {
        await Deno.mkdir(uploadDir, { recursive: true });
      } catch {
        // 目录可能已存在，忽略错误
      }

      // 保存文件
      const filename = `${Date.now()}_${file.name}`;
      const filepath = `${uploadDir}/${filename}`;
      await file.save(filepath);

      return response.json({
        success: true,
        message: "文件上传成功",
        file: {
          name: file.name,
          type: file.type,
          size: file.size,
          savedAs: filename,
        },
      });
    } catch (error) {
      return response.error(
        `文件保存失败: ${error instanceof Error ? error.message : String(error)}`,
        500,
      );
    }
  }

  // 处理多文件上传
  if (action === "upload-multiple" && ctx.method === "POST") {
    const files = ctx.files.files as UploadedFile[] | UploadedFile;

    if (!files) {
      return response.error("没有上传文件", 400);
    }

    const fileList = Array.isArray(files) ? files : [files];

    try {
      // 创建 uploads 目录
      const uploadDir = "./uploads";
      try {
        await Deno.mkdir(uploadDir, { recursive: true });
      } catch {
        // 目录可能已存在，忽略错误
      }

      const uploadedFiles = [];

      for (const file of fileList) {
        const filename = `${Date.now()}_${file.name}`;
        const filepath = `${uploadDir}/${filename}`;
        await file.save(filepath);

        uploadedFiles.push({
          name: file.name,
          type: file.type,
          size: file.size,
          savedAs: filename,
        });
      }

      return response.json({
        success: true,
        message: `成功上传 ${uploadedFiles.length} 个文件`,
        files: uploadedFiles,
      });
    } catch (error) {
      return response.error(
        `文件保存失败: ${error instanceof Error ? error.message : String(error)}`,
        500,
      );
    }
  }

  // 列出已上传的文件
  if (action === "list") {
    try {
      const uploadDir = "./uploads";
      const entries = Deno.readDir(uploadDir);
      const files = [];

      for await (const entry of entries) {
        if (entry.isFile) {
          const info = await Deno.stat(`${uploadDir}/${entry.name}`);
          files.push({
            name: entry.name,
            size: info.size,
            mtime: info.mtime?.toISOString(),
          });
        }
      }

      return response.json({
        files: files.sort((a, b) =>
          new Date(b.mtime || 0).getTime() - new Date(a.mtime || 0).getTime()
        ),
      });
    } catch (error) {
      return response.json({ files: [] });
    }
  }

  // 默认：展示上传表单
  return (
    <html>
      <head>
        <title>文件上传功能演示 - TSP</title>
        <meta charset="UTF-8" />
        <style>
          {`
          body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 900px;
            margin: 0 auto;
            padding: 40px 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
          }
          .container {
            background: white;
            border-radius: 16px;
            padding: 40px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
          }
          h1 {
            color: #333;
            margin-bottom: 10px;
            font-size: 2em;
          }
          .subtitle {
            color: #666;
            margin-bottom: 40px;
            font-size: 1.1em;
          }
          .upload-section {
            margin-bottom: 40px;
            padding: 30px;
            background: #f8f9fa;
            border-radius: 12px;
            border: 2px dashed #dee2e6;
          }
          .upload-section h2 {
            margin-top: 0;
            color: #495057;
            font-size: 1.5em;
            margin-bottom: 20px;
          }
          .form-group {
            margin-bottom: 20px;
          }
          label {
            display: block;
            margin-bottom: 8px;
            color: #495057;
            font-weight: 500;
          }
          input[type="file"] {
            width: 100%;
            padding: 12px;
            border: 2px solid #dee2e6;
            border-radius: 8px;
            font-size: 14px;
            background: white;
            cursor: pointer;
          }
          input[type="file"]:hover {
            border-color: #667eea;
          }
          button {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            padding: 12px 32px;
            border-radius: 8px;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: transform 0.2s, box-shadow 0.2s;
          }
          button:hover {
            transform: translateY(-2px);
            box-shadow: 0 8px 16px rgba(102, 126, 234, 0.4);
          }
          .file-list {
            margin-top: 30px;
          }
          .file-item {
            padding: 15px;
            background: white;
            border-radius: 8px;
            margin-bottom: 10px;
            border-left: 4px solid #667eea;
            display: flex;
            justify-content: space-between;
            align-items: center;
          }
          .file-info {
            flex: 1;
          }
          .file-name {
            font-weight: 600;
            color: #333;
            margin-bottom: 4px;
          }
          .file-meta {
            font-size: 14px;
            color: #666;
          }
          .btn-small {
            padding: 8px 16px;
            font-size: 14px;
            background: #6c757d;
          }
          .btn-small:hover {
            background: #5a6268;
          }
          .back-link {
            display: inline-block;
            margin-bottom: 20px;
            color: #667eea;
            text-decoration: none;
            font-weight: 500;
          }
          .back-link:hover {
            text-decoration: underline;
          }
          .info-box {
            background: #e7f3ff;
            border-left: 4px solid #2196f3;
            padding: 15px;
            border-radius: 8px;
            margin-bottom: 20px;
          }
          .info-box strong {
            color: #1976d2;
          }
        `}
        </style>
      </head>
      <body>
        <div class="container">
          <a href="/features" class="back-link">← 返回功能列表</a>

          <h1>📁 文件上传演示</h1>
          <p class="subtitle">支持单文件和多文件上传，自动保存到 uploads 目录</p>

          <div class="info-box">
            <strong>💡 提示：</strong>上传的文件会保存在服务器的 <code>./uploads/</code> 目录下，文件名会加上时间戳前缀以避免冲突。
          </div>

          {/* 单文件上传 */}
          <div class="upload-section">
            <h2>单文件上传</h2>
            <form method="POST" action="/features/file-upload?action=upload-single" enctype="multipart/form-data">
              <div class="form-group">
                <label for="single-file">选择文件：</label>
                <input
                  type="file"
                  id="single-file"
                  name="file"
                  required
                />
              </div>
              <button type="submit">上传文件</button>
            </form>
          </div>

          {/* 多文件上传 */}
          <div class="upload-section">
            <h2>多文件上传</h2>
            <form method="POST" action="/features/file-upload?action=upload-multiple" enctype="multipart/form-data">
              <div class="form-group">
                <label for="multiple-files">选择多个文件：</label>
                <input
                  type="file"
                  id="multiple-files"
                  name="files"
                  multiple
                  required
                />
              </div>
              <button type="submit">上传文件</button>
            </form>
          </div>

          {/* 文件列表 */}
          <div class="upload-section">
            <h2>已上传的文件</h2>
            <button onclick="loadFiles()" class="btn-small">刷新列表</button>
            <div id="file-list" class="file-list">
              <p style="color: #666; text-align: center; padding: 20px;">
                点击"刷新列表"查看已上传的文件
              </p>
            </div>
          </div>
        </div>

        <script dangerouslySetInnerHTML={{
          __html: `
          async function loadFiles() {
            const fileList = document.getElementById('file-list');
            fileList.innerHTML = '<p style="color: #666; text-align: center;">加载中...</p>';

            try {
              const res = await fetch('?action=list');
              const data = await res.json();

              if (data.files.length === 0) {
                fileList.innerHTML = '<p style="color: #666; text-align: center;">暂无上传文件</p>';
                return;
              }

              fileList.innerHTML = data.files.map(file => \`
                <div class="file-item">
                  <div class="file-info">
                    <div class="file-name">\${file.name}</div>
                    <div class="file-meta">
                      \${(file.size / 1024).toFixed(2)} KB •
                      \${new Date(file.mtime).toLocaleString('zh-CN')}
                    </div>
                  </div>
                </div>
              \`).join('');
            } catch (error) {
              fileList.innerHTML = '<p style="color: #dc3545; text-align: center;">加载失败: ' + error.message + '</p>';
            }
          }

          // 页面加载时自动加载文件列表
          loadFiles();

          // 拦截表单提交，使用 AJAX 处理
          document.querySelectorAll('form').forEach(form => {
            form.addEventListener('submit', async (e) => {
              e.preventDefault();

              const formData = new FormData(form);
              const submitBtn = form.querySelector('button[type="submit"]');
              const originalText = submitBtn.textContent;
              submitBtn.textContent = '上传中...';
              submitBtn.disabled = true;

              try {
                const res = await fetch(form.action, {
                  method: 'POST',
                  body: formData
                });

                const data = await res.json();

                if (data.success) {
                  alert(data.message);
                  form.reset();
                  loadFiles(); // 刷新文件列表
                } else {
                  alert('上传失败: ' + data.message);
                }
              } catch (error) {
                alert('上传失败: ' + error.message);
              } finally {
                submitBtn.textContent = originalText;
                submitBtn.disabled = false;
              }
            });
          });
          `
        }} />
      </body>
    </html>
  );
});
