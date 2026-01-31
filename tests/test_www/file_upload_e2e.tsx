/**
 * 文件上传 E2E 测试页面
 * 用于测试文件上传、保存等功能
 */

export default Page(async function(ctx, { response }) {
  const action = ctx.query.action || "demo";

  // 测试1: 单文件上传
  if (action === "upload-single" && ctx.method === "POST") {
    const file = ctx.files.file as UploadedFile;

    if (!file) {
      return response.json({
        success: false,
        error: "没有上传文件",
      });
    }

    try {
      // 创建测试上传目录
      const uploadDir = "./test-uploads";
      try {
        await Deno.mkdir(uploadDir, { recursive: true });
      } catch {
        // 目录可能已存在
      }

      // 保存文件
      const filename = `test_${Date.now()}_${file.name}`;
      const filepath = `${uploadDir}/${filename}`;
      await file.save(filepath);

      // 验证文件已保存
      const stat = await Deno.stat(filepath);

      return response.json({
        success: true,
        test: "单文件上传",
        file: {
          name: file.name,
          type: file.type,
          size: file.size,
          savedAs: filename,
          verifiedSize: stat.size,
        },
      });
    } catch (error) {
      return response.json({
        success: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  // 测试2: 多文件上传
  if (action === "upload-multiple" && ctx.method === "POST") {
    const files = ctx.files.files as UploadedFile[] | UploadedFile;

    if (!files) {
      return response.json({
        success: false,
        error: "没有上传文件",
      });
    }

    const fileList = Array.isArray(files) ? files : [files];

    try {
      const uploadDir = "./test-uploads";
      try {
        await Deno.mkdir(uploadDir, { recursive: true });
      } catch {
        // 目录可能已存在
      }

      const uploadedFiles = [];

      for (const file of fileList) {
        const filename = `test_multi_${Date.now()}_${file.name}`;
        const filepath = `${uploadDir}/${filename}`;
        await file.save(filepath);

        const stat = await Deno.stat(filepath);

        uploadedFiles.push({
          name: file.name,
          size: file.size,
          savedAs: filename,
          verifiedSize: stat.size,
        });
      }

      return response.json({
        success: true,
        test: "多文件上传",
        count: uploadedFiles.length,
        files: uploadedFiles,
      });
    } catch (error) {
      return response.json({
        success: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  // 测试3: 文件大小限制
  if (action === "upload-large" && ctx.method === "POST") {
    const file = ctx.files.file as UploadedFile;

    if (!file) {
      return response.json({
        success: false,
        error: "没有上传文件",
      });
    }

    // 返回文件大小信息（实际大小限制在解析时检查）
    return response.json({
      success: true,
      test: "文件大小检查",
      fileSize: file.size,
      note: "超过10MB的文件会在解析时被拒绝",
    });
  }

  // 测试4: 列出已上传的文件
  if (action === "list") {
    try {
      const uploadDir = "./test-uploads";
      const entries = Deno.readDir(uploadDir);
      const files = [];

      for await (const entry of entries) {
        if (entry.isFile && entry.name.startsWith("test_")) {
          const info = await Deno.stat(`${uploadDir}/${entry.name}`);
          files.push({
            name: entry.name,
            size: info.size,
          });
        }
      }

      return response.json({
        success: true,
        test: "文件列表",
        files: files.sort((a, b) => b.name.localeCompare(a.name)),
      });
    } catch (error) {
      return response.json({
        success: true,
        test: "文件列表",
        files: [],
        note: "目录不存在或为空",
      });
    }
  }

  // 测试5: 清理测试文件
  if (action === "cleanup") {
    try {
      const uploadDir = "./test-uploads";
      const entries = Deno.readDir(uploadDir);

      for await (const entry of entries) {
        if (entry.isFile && entry.name.startsWith("test_")) {
          await Deno.remove(`${uploadDir}/${entry.name}`);
        }
      }

      return response.json({
        success: true,
        test: "清理测试文件",
      });
    } catch (error) {
      return response.json({
        success: true,
        test: "清理测试文件",
        note: "目录不存在或已清空",
      });
    }
  }

  // 默认：显示测试说明
  return (
    <html>
      <head>
        <title>文件上传 E2E 测试</title>
        <meta charset="UTF-8" />
        <style>
          {`
          body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
          h1 { color: #333; }
          .test-section { margin: 30px 0; padding: 20px; border: 1px solid #ddd; border-radius: 8px; }
          .test-section h2 { margin-top: 0; color: #555; }
          code { background: #f5f5f5; padding: 2px 6px; border-radius: 3px; }
          .result { margin-top: 20px; padding: 15px; background: #f0f0f0; border-radius: 5px; }
        `}
        </style>
      </head>
      <body>
        <h1>📁 文件上传 E2E 测试</h1>

        <div class="test-section">
          <h2>测试项目</h2>
          <ul>
            <li>单文件上传 - POST ?action=upload-single</li>
            <li>多文件上传 - POST ?action=upload-multiple</li>
            <li>文件大小检查 - POST ?action=upload-large</li>
            <li>文件列表 - GET ?action=list</li>
            <li>清理测试文件 - GET ?action=cleanup</li>
          </ul>
        </div>

        <div class="test-section">
          <h2>E2E 测试说明</h2>
          <p>此页面用于 E2E 测试文件上传功能：</p>
          <ul>
            <li>上传的文件保存在 <code>./test-uploads/</code> 目录</li>
            <li>所有测试文件以 <code>test_</code> 前缀命名</li>
            <li>测试完成后应清理测试文件</li>
          </ul>
        </div>
      </body>
    </html>
  );
});
