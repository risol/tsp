/**
 * Posts list API - For E2E testing
 * Demonstrates query validation functionality
 */

export default Page(async function(ctx, { createZod, query, response }) {
  const z = await createZod();
  const schema = z.object({
    page: z.coerce.number().min(1).default(1),
    limit: z.coerce.number().min(1).max(100).default(10),
  });

  try {
    const { page, limit } = query(schema);
    return response.json({
      success: true,
      page,
      limit,
      total: 100,
      data: [],
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
