/**
 * File Upload E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "../run_e2e_tests.ts";

export function getUploadTests() {
  return [
    {
      name: "file upload - File upload functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("File Upload Test");

        // Test 1: Single file upload
        console.log(`  ${COLORS.dim}Test 1: Single file upload${COLORS.reset}`);

        // Create test file content
        const testContent = "Hello from E2E test!";

        // Create multipart/form-data (note: boundary should not include prefix --)
        const boundary = "E2ETestBoundary" + Date.now().toString(36);
        const multipartData = [
          `--${boundary}`,
          'Content-Disposition: form-data; name="file"; filename="test.txt"',
          "Content-Type: text/plain",
          "",
          testContent,
          `--${boundary}--`,
          "",
        ].join("\r\n");

        const uploadResponse = await fetch(
          `http://localhost:${TEST_PORT}/file_upload_e2e.tsp?action=upload-single`,
          {
            method: "POST",
            headers: {
              "Content-Type": `multipart/form-data; boundary=${boundary}`,
            },
            body: multipartData,
          },
        );

        assertEquals(uploadResponse.status, 200);

        const uploadResult = await uploadResponse.json();
        if (!uploadResult.success) {
          throw new Error(`Single file upload failed: ${uploadResult.error}`);
        }
        console.log(
          `  ${COLORS.dim}File name: ${uploadResult.file.name}${COLORS.reset}`,
        );
        console.log(
          `  ${COLORS.dim}File size: ${uploadResult.file.size} bytes${COLORS.reset}`,
        );
        printTestResult("Single file upload", true);

        // Test 2: Multi-file upload
        console.log(`  ${COLORS.dim}Test 2: Multi-file upload${COLORS.reset}`);

        const testContent2 = "Second test file";

        const multipartData2 = [
          `--${boundary}`,
          'Content-Disposition: form-data; name="files"; filename="test1.txt"',
          "Content-Type: text/plain",
          "",
          testContent,
          `--${boundary}`,
          'Content-Disposition: form-data; name="files"; filename="test2.txt"',
          "Content-Type: text/plain",
          "",
          testContent2,
          `--${boundary}--`,
          "",
        ].join("\r\n");

        const multiUploadResponse = await fetch(
          `http://localhost:${TEST_PORT}/file_upload_e2e.tsp?action=upload-multiple`,
          {
            method: "POST",
            headers: {
              "Content-Type": `multipart/form-data; boundary=${boundary}`,
            },
            body: multipartData2,
          },
        );

        assertEquals(multiUploadResponse.status, 200);

        const multiUploadResult = await multiUploadResponse.json();
        if (!multiUploadResult.success) {
          throw new Error(`Multi-file upload failed: ${multiUploadResult.error}`);
        }
        console.log(
          `  ${COLORS.dim}Uploaded files count: ${multiUploadResult.count}${COLORS.reset}`,
        );
        printTestResult("Multi-file upload", true);

        // Test 3: View file list
        console.log(`  ${COLORS.dim}Test 3: File list${COLORS.reset}`);

        const listResponse = await fetch(
          `http://localhost:${TEST_PORT}/file_upload_e2e.tsp?action=list`,
        );

        assertEquals(listResponse.status, 200);

        const listResult = await listResponse.json();
        console.log(
          `  ${COLORS.dim}Files in list: ${listResult.files.length}${COLORS.reset}`,
        );

        if (listResult.files.length < 3) {
          throw new Error(
            `Insufficient files in list: expected >= 3, got ${listResult.files.length}`,
          );
        }
        printTestResult("File list", true);

        // Test 4: Cleanup test files
        console.log(`  ${COLORS.dim}Test 4: Cleanup test files${COLORS.reset}`);

        const cleanupResponse = await fetch(
          `http://localhost:${TEST_PORT}/file_upload_e2e.tsp?action=cleanup`,
        );

        assertEquals(cleanupResponse.status, 200);

        const cleanupResult = await cleanupResponse.json();
        if (!cleanupResult.success) {
          throw new Error(
            `Cleanup test files failed: ${cleanupResult.error || "unknown error"}`,
          );
        }
        printTestResult("Cleanup test files", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
