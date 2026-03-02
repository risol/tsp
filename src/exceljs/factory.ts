import ExcelJS from "npm:exceljs@^4.4.0";

/**
 * ExcelJS factory function
 * Returns ExcelJS library instance, can use its full API directly
 *
 * @returns ExcelJS library
 *
 * @example
 * ```tsx
 * export default Page(async function(ctx, { createExcelJS, response }) {
 *   const ExcelJS = await createExcelJS();
 *
 *   // Read Excel file
 *   const workbook = new ExcelJS.Workbook();
 *   await workbook.xlsx.readFile('./data.xlsx');
 *
 *   // Get worksheet
 *   const worksheet = workbook.getWorksheet('Sheet1');
 *   const data: any[] = [];
 *   worksheet?.eachRow((row) => {
 *     data.push(row.values);
 *   });
 *
 *   return response.json({ data });
 * });
 * ```
 */
export async function createExcelJS(): Promise<typeof ExcelJS> {
  return ExcelJS;
}
