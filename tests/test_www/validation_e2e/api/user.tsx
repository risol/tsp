/**
 * User creation API - For E2E testing
 * Demonstrates body validation functionality
 */

export default Page(async function(ctx, { createZod, body, response }) {
  const z = await createZod();
  const userSchema = z.object({
    name: z.string().min(2, "Name must be at least 2 characters"),
    email: z.string().email("Invalid email format"),
    age: z.coerce.number().min(1).max(150).optional(),
  });

  try {
    const userData = body(userSchema);
    return response.json({ success: true, data: userData });
  } catch (error) {
    if (error instanceof z.ZodError) {
      return response.json(
        {
          success: false,
          message: "Data validation failed",
          errors: error.errors.map((e) => ({
            path: e.path.map(String),
            message: e.message,
            code: e.code,
          })),
        },
        400,
      );
    }
    throw error;
  }
});
