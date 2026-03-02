/**
 * Tags management API - For E2E testing
 * Demonstrates array field validation functionality
 */

export default Page(async function(ctx, { createZod, body, response }) {
  const z = await createZod();
  const tagsSchema = z.object({
    tags: z.array(z.string()).min(1),
    categories: z.array(z.string()).optional(),
  });

  try {
    const { tags, categories } = body(tagsSchema);
    return response.json({
      success: true,
      tags,
      categories,
    });
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
