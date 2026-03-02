/**
 * Zod validator E2E test
 * Test body, query, zod dependencies in real usage scenarios
 */

export default Page(async function(ctx, { testHelper, response }) {
  // Clear previous test results
  testHelper.clear();

  const baseURL = `http://localhost:${ctx.url.port}`;

  // ============================================
  // Test 1: Normal data validation (POST request, JSON data)
  // ============================================
  await testHelper.test(
    "Scenario 1: Normal data validation (POST request, JSON data)",
    async () => {
      const resp = await fetch(`${baseURL}/validation_e2e/api/user`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: "Alice",
          email: "alice@example.com",
          age: 25,
        }),
      });

      const data = await resp.json();
      testHelper.assertEqual(data.success, true);
      testHelper.assertEqual(data.data.name, "Alice");
      testHelper.assertEqual(data.data.email, "alice@example.com");
      testHelper.assertEqual(data.data.age, 25);
    },
  );

  // ============================================
  // Test 2: Type error validation (string passed to number field)
  // ============================================
  await testHelper.test(
    "Scenario 2: Type error validation (string passed to number field)",
    async () => {
      const resp = await fetch(`${baseURL}/validation_e2e/api/user`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: "Bob",
          email: "bob@example.com",
          age: "not_a_number", // Should be number
        }),
      });

      const data = await resp.json();
      testHelper.assertEqual(data.success, false);
      testHelper.assertEqual(data.message, "Data validation failed");
      testHelper.assertTrue(Array.isArray(data.errors));
    },
  );

  // ============================================
  // Test 3: Format error validation (invalid email)
  // ============================================
  await testHelper.test(
    "Scenario 3: Format error validation (invalid email)",
    async () => {
      const resp = await fetch(`${baseURL}/validation_e2e/api/user`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: "Charlie",
          email: "invalid_email", // Invalid email format
          age: 30,
        }),
      });

      const data = await resp.json();
      testHelper.assertEqual(data.success, false);
      testHelper.assertTrue(
        data.errors.some((e: any) => e.path.includes("email")),
      );
    },
  );

  // ============================================
  // Test 4: Query parameter validation (pagination parameters)
  // ============================================
  await testHelper.test(
    "Scenario 4: Query parameter validation (pagination parameters)",
    async () => {
      const resp = await fetch(
        `${baseURL}/validation_e2e/api/posts?page=2&limit=20`,
      );

      const data = await resp.json();
      testHelper.assertEqual(data.success, true);
      testHelper.assertEqual(data.page, 2);
      testHelper.assertEqual(data.limit, 20);
    },
  );

  // ============================================
  // Test 5: Nested object validation
  // ============================================
  await testHelper.test(
    "Scenario 5: Nested object validation",
    async () => {
      const resp = await fetch(`${baseURL}/validation_e2e/api/config`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          theme: {
            mode: "dark",
            primaryColor: "#007bff",
          },
          language: "en",
          notifications: {
            email: true,
            push: false,
          },
        }),
      });

      const data = await resp.json();
      testHelper.assertEqual(data.success, true);
      testHelper.assertEqual(data.config.theme.mode, "dark");
      testHelper.assertEqual(data.config.theme.primaryColor, "#007bff");
      testHelper.assertEqual(data.config.language, "en");
      testHelper.assertEqual(data.config.notifications.email, true);
      testHelper.assertEqual(data.config.notifications.push, false);
    },
  );

  // ============================================
  // Test 6: Array field validation
  // ============================================
  await testHelper.test(
    "Scenario 6: Array field validation",
    async () => {
      const resp = await fetch(`${baseURL}/validation_e2e/api/tags`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tags: ["typescript", "zod", "validation"],
          categories: ["tech", "programming"],
        }),
      });

      const data = await resp.json();
      testHelper.assertEqual(data.success, true);
      testHelper.assertTrue(Array.isArray(data.tags));
      testHelper.assertEqual(data.tags.length, 3);
      testHelper.assertTrue(data.tags.includes("typescript"));
    },
  );

  // ============================================
  // Test 7: Missing required fields
  // ============================================
  await testHelper.test(
    "Scenario 7: Missing required fields",
    async () => {
      const resp = await fetch(`${baseURL}/validation_e2e/api/user`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: "David",
          // email missing
        }),
      });

      const data = await resp.json();
      testHelper.assertEqual(data.success, false);
      testHelper.assertTrue(
        data.errors.some((e: any) => e.path.includes("email")),
      );
    },
  );

  // ============================================
  // Test 8: Optional field handling
  // ============================================
  await testHelper.test(
    "Scenario 8: Optional field handling",
    async () => {
      const resp = await fetch(`${baseURL}/validation_e2e/api/user`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: "Eve",
          email: "eve@example.com",
          // age is optional
        }),
      });

      const data = await resp.json();
      testHelper.assertEqual(data.success, true);
      testHelper.assertEqual(data.data.name, "Eve");
      testHelper.assertEqual(data.data.email, "eve@example.com");
      testHelper.assertEqual(data.data.age, undefined);
    },
  );

  // Get test results
  const results = testHelper.getResults();

  // Return JSON format results
  return response.json(results);
});
