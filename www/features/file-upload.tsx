import { Layout } from "../components/Layout.tsx";

export default async function (context: PageContext) {
  const { method, files } = context;

  // 处理文件上传
  let uploadedFiles: Array<{ original: string; saved: string; size: number }> = [];

  if (method === "POST" && files && files.file) {
    const { generateUniqueFilename } = await import("../../src/files.ts");

    const uploadFile = async (file: any) => {
      // 使用 nanoid 生成唯一文件名
      const uniqueFilename = generateUniqueFilename(file.name);
      const savePath = `./uploads/${uniqueFilename}`;

      // 确保上传目录存在
      await Deno.mkdir("./uploads", { recursive: true });

      // 保存文件
      await file.save(savePath);

      return {
        original: file.name,
        saved: uniqueFilename,
        size: file.size,
      };
    };

    // 处理单个文件或文件数组
    if (Array.isArray(files.file)) {
      for (const file of files.file) {
        uploadedFiles.push(await uploadFile(file));
      }
    } else {
      uploadedFiles.push(await uploadFile(files.file));
    }
  }

  return (
    <Layout title="文件上传 - TSP" description="使用 nanoid 的文件上传演示">
      <h1 style={{ fontSize: "32px", marginBottom: "24px" }}>
        📁 文件上传（使用 nanoid）
      </h1>
      <p style={{ color: "#64748b", marginBottom: "32px" }}>
        文件上传后使用 <code>nanoid</code> 生成唯一的文件名，避免冲突
      </p>

      {/* Upload Result */}
      {uploadedFiles.length > 0 && (
        <div
          className="card"
          style={{
            background: "#d1fae5",
            border: "2px solid #10b981",
            marginBottom: "32px",
          }}
        >
          <h3 style={{ color: "#065f46", marginBottom: "12px" }}>
            ✅ 上传成功！
          </h3>
          <div
            style={{
              background: "#064e3b",
              padding: "16px",
              borderRadius: "4px",
              marginTop: "16px",
            }}
          >
            {uploadedFiles.map((f, i) => (
              <div key={i} style={{ marginBottom: i < uploadedFiles.length - 1 ? "12px" : "0" }}>
                <div style={{ color: "#34d399", fontSize: "13px", marginBottom: "4px" }}>
                  文件 #{i + 1}
                </div>
                <div style={{ color: "#fff", fontSize: "14px" }}>
                  原始名称: <span style={{ color: "#fbbf24" }}>{f.original}</span>
                </div>
                <div style={{ color: "#fff", fontSize: "14px" }}>
                  保存为: <span style={{ color: "#60a5fa" }}>{f.saved}</span>
                </div>
                <div style={{ color: "#fff", fontSize: "14px" }}>
                  大小: <span style={{ color: "#f472b6" }}>
                    {(f.size / 1024).toFixed(2)} KB
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Upload Form */}
      <div className="section">
        <h2 className="section-title">文件上传表单</h2>
        <div className="card">
          <form method="POST" enctype="multipart/form-data" style={{ maxWidth: "500px" }}>
            <div style={{ marginBottom: "16px" }}>
              <label
                style={{
                  display: "block",
                  fontWeight: "600",
                  marginBottom: "8px",
                }}
              >
                选择文件：
              </label>
              <input
                type="file"
                name="file"
                required
                style={{
                  width: "100%",
                  padding: "10px",
                  border: "1px solid #ddd",
                  borderRadius: "4px",
                  fontSize: "14px",
                }}
              />
            </div>
            <div style={{ marginBottom: "16px" }}>
              <label style={{ display: "flex", alignItems: "center", fontSize: "14px" }}>
                <input type="checkbox" name="multiple" value="1" style={{ marginRight: "8px" }} />
                允许多文件上传（使用同名字段）
              </label>
            </div>
            <button type="submit" className="btn btn-primary">
              上传文件
            </button>
          </form>
        </div>
      </div>

      {/* Usage Example */}
      <div className="section">
        <h2 className="section-title">使用示例</h2>
        <div className="code-block">
          {`// 导入 nanoid 文件名生成函数
import { generateUniqueFilename } from "./src/files.ts";

// 处理文件上传
export default async function (context: PageContext) {
  const { files } = context;

  if (files && files.file) {
    const file = files.file;

    // 使用 nanoid 生成唯一文件名
    // 例如：photo.jpg → photo_V1StGXR8_Z5jdHi6B-myT.jpg
    const uniqueFilename = generateUniqueFilename(file.name);

    // 保存文件
    await file.save(\`./uploads/\${uniqueFilename}\`);
  }

  return <div>...</div>;
}`}
        </div>
      </div>

      <div style={{ marginTop: "40px" }}>
        <a href="/features" className="btn btn-secondary">← 返回功能列表</a>
      </div>
    </Layout>
  );
}
