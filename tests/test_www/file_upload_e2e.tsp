/**
 * File Upload E2E Test Page - TSP File
 * Used to test file upload, save and other features
 */

export default Page(async function(ctx, { response, query, createZod }) {
  const z = await createZod();
  const { action = 'demo' } = query(z.object({
    action: z.string().default('demo')
  }));

  // Test 1: Single file upload
  if (action === "upload-single" && ctx.method === "POST") {
    const file = ctx.files.file as UploadedFile;

    if (!file) {
      return response.json({
        success: false,
        error: "No file uploaded",
      });
    }

    try {
      // Create test upload directory
      const uploadDir = "./test-uploads";
      try {
        await Deno.mkdir(uploadDir, { recursive: true });
      } catch {
        // Directory may already exist
      }

      // Save file
      const filename = `test_${Date.now()}_${file.name}`;
      const filepath = `${uploadDir}/${filename}`;
      await file.save(filepath);

      // Verify file was saved
      const stat = await Deno.stat(filepath);

      return response.json({
        success: true,
        test: "Single file upload",
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

  // Test 2: Multiple file upload
  if (action === "upload-multiple" && ctx.method === "POST") {
    const files = ctx.files.files as UploadedFile[] | UploadedFile;

    if (!files) {
      return response.json({
        success: false,
        error: "No file uploaded",
      });
    }

    const fileList = Array.isArray(files) ? files : [files];

    try {
      const uploadDir = "./test-uploads";
      try {
        await Deno.mkdir(uploadDir, { recursive: true });
      } catch {
        // Directory may already exist
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
        test: "Multiple file upload",
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

  // Test 3: File size limit
  if (action === "upload-large" && ctx.method === "POST") {
    const file = ctx.files.file as UploadedFile;

    if (!file) {
      return response.json({
        success: false,
        error: "No file uploaded",
      });
    }

    // Return file size info (actual size limit checked during parsing)
    return response.json({
      success: true,
      test: "File size check",
      fileSize: file.size,
      note: "Files over 10MB will be rejected during parsing",
    });
  }

  // Test 4: List uploaded files
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
        test: "File list",
        files: files.sort((a, b) => b.name.localeCompare(a.name)),
      });
    } catch (error) {
      return response.json({
        success: true,
        test: "File list",
        files: [],
        note: "Directory does not exist or is empty",
      });
    }
  }

  // Test 5: Clean up test files
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
        test: "Clean up test files",
      });
    } catch (error) {
      return response.json({
        success: true,
        test: "Clean up test files",
        note: "Directory does not exist or is already empty",
      });
    }
  }

  // Default: Show test instructions
  return (
    <html>
      <head>
        <title>File Upload E2E Test</title>
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
        <h1>File Upload E2E Test</h1>

        <div class="test-section">
          <h2>Test Items</h2>
          <ul>
            <li>Single file upload - POST ?action=upload-single</li>
            <li>Multiple file upload - POST ?action=upload-multiple</li>
            <li>File size check - POST ?action=upload-large</li>
            <li>File list - GET ?action=list</li>
            <li>Clean up test files - GET ?action=cleanup</li>
          </ul>
        </div>

        <div class="test-section">
          <h2>E2E Test Instructions</h2>
          <p>This page is used for E2E testing file upload functionality:</p>
          <ul>
            <li>Uploaded files are saved in <code>./test-uploads/</code> directory</li>
            <li>All test files are named with <code>test_</code> prefix</li>
            <li>Test files should be cleaned up after testing</li>
          </ul>
        </div>
      </body>
    </html>
  );
});
