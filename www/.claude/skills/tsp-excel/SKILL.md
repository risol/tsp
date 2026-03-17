---
name: tsp-excel
description: Excel file operations using ExcelJS. Use when reading or writing Excel files, exporting data, or generating reports.
---

# TSP Excel

Use this skill for Excel file operations in TSP.

## ExcelJS Usage

```typescript
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();

  // ====== Write Excel ======
  const workbook = new ExcelJS.Workbook();
  workbook.creator = 'TSP';
  workbook.created = new Date();

  // Add worksheet
  const sheet = workbook.addWorksheet('Users');

  // Add headers
  sheet.addRow(['ID', 'Name', 'Email', 'Created At']);

  // Add data
  const users = [
    { id: 1, name: 'John', email: 'john@example.com' },
    { id: 2, name: 'Jane', email: 'jane@example.com' }
  ];

  users.forEach(user => {
    sheet.addRow([user.id, user.name, user.email, new Date()]);
  });

  // Write to file
  await workbook.xlsx.writeFile('./users.xlsx');

  // Or write to buffer (for download)
  const buffer = await workbook.xlsx.writeBuffer();


  // ====== Read Excel ======
  const readWorkbook = new ExcelJS.Workbook();
  await readWorkbook.xlsx.readFile('./users.xlsx');

  const readSheet = readWorkbook.getWorksheet('Users');
  const data: any[] = [];

  readSheet?.eachRow((row, rowNumber) => {
    if (rowNumber > 1) { // Skip header
      data.push({
        id: row.getCell(1).value,
        name: row.getCell(2).value,
        email: row.getCell(3).value
      });
    }
  });

  // Or use JSON
  const jsonData = readSheet?.getSheetJson();


  // ====== Advanced Features ======

  // Styling
  sheet.getCell('A1').font = { bold: true, color: { argb: 'FF0000' } };
  sheet.getCell('A1').fill = {
    type: 'pattern',
    pattern: 'solid',
    fgColor: { argb: 'FFFF00' }
  };
  sheet.getCell('A1').border = {
    top: { style: 'thin' },
    left: { style: 'thin' },
    bottom: { style: 'thin' },
    right: { style: 'thin' }
  };

  // Column width
  sheet.getColumn(1).width = 20;
  sheet.getColumn(2).width = 30;

  // Freeze panes
  sheet.views = [{ state: 'frozen', xSplit: 1, ySplit: 1 }];

  // Multiple worksheets
  workbook.addWorksheet('Products');
  workbook.addWorksheet('Orders');

  return response.json({ success: true, rowCount: data.length });
});
```

## Key Classes

| Class | Description |
|-------|-------------|
| `Workbook` | Excel file container |
| `Worksheet` | Single sheet |
| `Row` | Row of data |
| `Cell` | Individual cell |

## Common Operations

| Operation | Method |
|-----------|--------|
| Add row | `sheet.addRow([...])` |
| Add multiple rows | `sheet.addRows([...])` |
| Get cell | `sheet.getCell('A1')` |
| Get column | `sheet.getColumn(1)` |
| Write file | `workbook.xlsx.writeFile(path)` |
| Write buffer | `workbook.xlsx.writeBuffer()` |
| Read file | `workbook.xlsx.readFile(path)` |

## Best Practices

- Use `createExcelJS` without arguments - it loads the library automatically
- For downloads, use `writeBuffer()` and return with appropriate headers
- For exports to users, set response Content-Type to `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
- Use styling sparingly - it adds file size
