/**
 * ExcelJS E2E Test Page - TSP File
 * Test ExcelJS full functionality (using native API)
 */

export default Page(async function(ctx, { createExcelJS, createZod, response }) {
  const z = await createZod();
  const results: Record<string, unknown> = {};
  const ExcelJS = await createExcelJS();

  // Test file directory (relative to server root)
  const testDir = "./tests/test_www";

  // Test 1: Basic write and read
  try {
    const testData = [
      { name: "Alice", age: 25, email: "alice@example.com" },
      { name: "Bob", age: 30, email: "bob@example.com" },
      { name: "Charlie", age: 35, email: "charlie@example.com" },
    ];

    // Create workbook and write
    const workbook = new ExcelJS.Workbook();
    const worksheet = workbook.addWorksheet("Users");

    // Add headers
    worksheet.columns = [
      { header: "name", key: "name", width: 20 },
      { header: "age", key: "age", width: 10 },
      { header: "email", key: "email", width: 30 },
    ];

    // Add data
    testData.forEach((row) => worksheet.addRow(row));

    const testFile = `${testDir}/test_exceljs_write.xlsx`;
    await workbook.xlsx.writeFile(testFile);

    // Read file
    const readWorkbook = new ExcelJS.Workbook();
    await readWorkbook.xlsx.readFile(testFile);
    const readWorksheet = readWorkbook.getWorksheet("Users");

    const readData: any[] = [];
    readWorksheet?.eachRow((row) => {
      readData.push(row.values);
    });

    results.test1_write_read = {
      status: readData.length >= 3 ? "✓ PASSED" : "✗ FAILED",
      data: { count: readData.length - 1 }, // Subtract header row
    };
  } catch (error) {
    results.test1_write_read = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 2: Zod validation read
  try {
    const UserSchema = z.object({
      name: z.string(),
      age: z.number(),
      email: z.string().email(),
    });

    const testFile = `${testDir}/test_exceljs_schema.xlsx`;
    const testData = [
      { name: "David", age: 28, email: "david@example.com" },
    ];

    // Write data
    const workbook = new ExcelJS.Workbook();
    const worksheet = workbook.addWorksheet("Sheet1");
    worksheet.addRow(["name", "age", "email"]);
    testData.forEach((row) => worksheet.addRow([row.name, row.age, row.email]));
    await workbook.xlsx.writeFile(testFile);

    // Read and validate
    const readWorkbook = new ExcelJS.Workbook();
    await readWorkbook.xlsx.readFile(testFile);
    const readWorksheet = readWorkbook.getWorksheet("Sheet1");

    const readData: any[] = [];
    readWorksheet?.eachRow((row, rowNumber) => {
      if (rowNumber > 1) { // Skip header
        readData.push({
          name: row.getCell(1).value,
          age: row.getCell(2).value,
          email: row.getCell(3).value,
        });
      }
    });

    // Use Zod validation
    const validated = UserSchema.parse(readData[0]);

    results.test2_zod_validation = {
      status: validated ? "✓ PASSED" : "✗ FAILED",
      data: { validated },
    };
  } catch (error) {
    results.test2_zod_validation = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 3: Worksheet info retrieval
  try {
    const workbook = new ExcelJS.Workbook();
    workbook.addWorksheet("Sheet1");
    workbook.addWorksheet("Sheet2");

    const testFile = `${testDir}/test_exceljs_sheets.xlsx`;
    await workbook.xlsx.writeFile(testFile);

    // Read worksheet info
    const readWorkbook = new ExcelJS.Workbook();
    await readWorkbook.xlsx.readFile(testFile);

    const sheets = readWorkbook.worksheets.map((ws) => ({
      name: ws.name,
      rowCount: ws.rowCount,
      columnCount: ws.columnCount,
    }));

    results.test3_sheets_info = {
      status: sheets.length === 2 ? "✓ PASSED" : "✗ FAILED",
      data: {
        sheetCount: sheets.length,
        sheets,
      },
    };
  } catch (error) {
    results.test3_sheets_info = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 4: Export as Buffer
  try {
    const workbook = new ExcelJS.Workbook();
    const worksheet = workbook.addWorksheet("Test");
    worksheet.addRow(["Name", "Value"]);
    worksheet.addRow(["Test", 123]);

    const buffer = await workbook.xlsx.writeBuffer();

    results.test4_export_buffer = {
      status: buffer && buffer.length > 0 ? "✓ PASSED" : "✗ FAILED",
      data: {
        bufferLength: buffer?.length || 0,
      },
    };
  } catch (error) {
    results.test4_export_buffer = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 5: Export as base64
  try {
    const workbook = new ExcelJS.Workbook();
    const worksheet = workbook.addWorksheet("Test");
    worksheet.addRow(["Name", "Value"]);
    worksheet.addRow(["Test", 456]);

    const buffer = await workbook.xlsx.writeBuffer();
    const base64 = btoa(String.fromCharCode(...new Uint8Array(buffer)));

    results.test5_export_base64 = {
      status: base64.length > 0 ? "✓ PASSED" : "✗ FAILED",
      data: {
        base64Length: base64.length,
      },
    };
  } catch (error) {
    results.test5_export_base64 = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 6: Template fill
  try {
    // Create template
    const templateWorkbook = new ExcelJS.Workbook();
    const templateWorksheet = templateWorkbook.addWorksheet("Template");
    templateWorksheet.addRow(["Name: {{name}}"]);
    templateWorksheet.addRow(["Age: {{age}}"]);
    templateWorksheet.addRow(["Email: {{email}}"]);

    const templateFile = `${testDir}/test_exceljs_template.xlsx`;
    await templateWorkbook.xlsx.writeFile(templateFile);

    // Fill template
    const filledWorkbook = new ExcelJS.Workbook();
    await filledWorkbook.xlsx.readFile(templateFile);

    filledWorkbook.eachSheet((worksheet) => {
      worksheet.eachRow((row) => {
        row.eachCell((cell) => {
          if (typeof cell.value === "string") {
            let value = cell.value;
            value = value.replace(/\{\{name\}\}/g, "Eve");
            value = value.replace(/\{\{age\}\}/g, "27");
            value = value.replace(/\{\{email\}\}/g, "eve@example.com");
            cell.value = value;
          }
        });
      });
    });

    const filledFile = `${testDir}/test_exceljs_filled.xlsx`;
    await filledWorkbook.xlsx.writeFile(filledFile);

    results.test6_template_fill = {
      status: "✓ PASSED",
      data: {
        message: "Template fill successful",
      },
    };
  } catch (error) {
    results.test6_template_fill = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 7: Cell operations
  try {
    const workbook = new ExcelJS.Workbook();
    const worksheet = workbook.addWorksheet("Test");

    worksheet.getCell("A1").value = "Hello ExcelJS!";
    worksheet.getCell("B1").value = 123;
    worksheet.getCell("C1").value = true;

    const testFile = `${testDir}/test_exceljs_cells.xlsx`;
    await workbook.xlsx.writeFile(testFile);

    results.test7_cell_operations = {
      status: "✓ PASSED",
      data: {
        message: "Cell operations successful",
      },
    };
  } catch (error) {
    results.test7_cell_operations = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 8: Styling
  try {
    const workbook = new ExcelJS.Workbook();
    const worksheet = workbook.addWorksheet("Products");

    // Add headers
    worksheet.columns = [
      { header: "Product", key: "product", width: 20 },
      { header: "Price", key: "price", width: 10 },
    ];

    // Add data
    worksheet.addRow({ product: "Apple", price: 1.5 });
    worksheet.addRow({ product: "Banana", price: 0.8 });

    // Set header styles
    const headerRow = worksheet.getRow(1);
    headerRow.font = { bold: true, size: 14, color: { argb: "FFFFFFFF" } };
    headerRow.fill = {
      type: "pattern",
      pattern: "solid",
      fgColor: { argb: "FF4472C4" },
    };
    headerRow.alignment = { horizontal: "center", vertical: "middle" };

    await workbook.xlsx.writeFile(`${testDir}/test_exceljs_style.xlsx`);

    results.test8_styling = {
      status: "✓ PASSED",
      data: {
        message: "Styling successful",
      },
    };
  } catch (error) {
    results.test8_styling = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 9: Worksheet operations
  try {
    const workbook = new ExcelJS.Workbook();
    workbook.addWorksheet("Sheet1");
    workbook.addWorksheet("Sheet2");

    const testFile = `${testDir}/test_exceljs_worksheet_ops.xlsx`;
    await workbook.xlsx.writeFile(testFile);

    // Rename
    const renameWorkbook = new ExcelJS.Workbook();
    await renameWorkbook.xlsx.readFile(testFile);
    const sheet2 = renameWorkbook.getWorksheet("Sheet2");
    if (sheet2) sheet2.name = "RenamedSheet";
    await renameWorkbook.xlsx.writeFile(testFile);

    // Delete
    const deleteWorkbook = new ExcelJS.Workbook();
    await deleteWorkbook.xlsx.readFile(testFile);
    const sheetToDelete = deleteWorkbook.getWorksheet("RenamedSheet");
    if (sheetToDelete) deleteWorkbook.removeWorksheet(sheetToDelete.id);
    await deleteWorkbook.xlsx.writeFile(testFile);

    // Verify
    const verifyWorkbook = new ExcelJS.Workbook();
    await verifyWorkbook.xlsx.readFile(testFile);

    const sheets = verifyWorkbook.worksheets.map((ws) => ws.name);

    results.test9_worksheet_ops = {
      status: sheets.length === 1 ? "✓ PASSED" : "✗ FAILED",
      data: {
        remainingSheets: sheets.length,
        sheetName: sheets[0],
      },
    };
  } catch (error) {
    results.test9_worksheet_ops = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Calculate test results
  const passed = Object.values(results).filter((r: any) => r.status === "✓ PASSED").length;
  const total = Object.keys(results).length;

  return response.json({
    testSuite: "ExcelJS E2E Tests",
    total,
    passed,
    failed: total - passed,
    results,
  });
});
