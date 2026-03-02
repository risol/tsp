/**
 * Config update API - For E2E testing
 * Demonstrates nested object validation functionality
 */

export default Page(async function(ctx, { createZod, body, response }) {
  const z = await createZod();
  const configSchema = z.object({
    theme: z.object({
      mode: z.enum(["light", "dark"]),
      primaryColor: z.string(),
    }),
    language: z.string(),
    notifications: z.object({
      email: z.boolean(),
      push: z.boolean(),
    }),
  });

  try {
    const config = body(configSchema);
    return response.json({ success: true, config });
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
