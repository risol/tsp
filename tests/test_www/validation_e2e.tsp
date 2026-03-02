/**
 * Validation test page - TSP file
 */

export default Page(async function(ctx, { createZod, response }) {
  const z = await createZod();
  // Test if Zod validator is available
  try {
    // Define validation schema
    const UserSchema = z.object({
      name: z.string(),
      age: z.number(),
      email: z.string().email(),
    });

    // Validate valid data
    const validData = {
      name: "Zhang San",
      age: 25,
      email: "test@example.com",
    };

    const validated = UserSchema.parse(validData);

    // Test validation failure
    try {
      UserSchema.parse({ name: "Li Si", age: "invalid" });
    } catch (e) {
      // Expected failure
    }

    // Return test results
    return response.json({
      total: 2,
      passed: 2,
      tests: [
        { name: "Zod available", passed: true },
        { name: "Schema validation", passed: validated.name === "Zhang San" },
      ],
    });
  } catch (error) {
    return response.json({
      total: 1,
      passed: 0,
      error: (error as Error).message,
    }, 500);
  }
});
