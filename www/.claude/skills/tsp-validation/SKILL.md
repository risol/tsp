---
name: tsp-validation
description: Request validation in TSP using Zod, body(), and query(). Use when validating HTTP request input or transforming query parameters.
---

# TSP Validation

Use this skill for request validation in TSP.

## body() - Validate Request Body

```typescript
export default Page(async function(ctx, { body, createZod, response }) {
  const z = await createZod();

  // Validate POST body
  const data = body(z.object({
    name: z.string().min(2).max(100),
    email: z.string().email(),
    age: z.coerce.number().min(0).max(150).optional(),
    role: z.enum(['admin', 'user', 'guest']).default('user')
  }));

  // data is fully typed and validated
  // age is automatically coerced from string to number

  return response.json({ success: true, data });
});
```

## query() - Validate Query Parameters

```typescript
export default Page(async function(ctx, { query, createZod, response }) {
  const z = await createZod();

  // Validate query string
  const params = query(z.object({
    page: z.coerce.number().min(1).default(1),
    pageSize: z.coerce.number().min(1).max(100).default(10),
    keyword: z.string().optional(),
    sort: z.enum(['asc', 'desc']).default('asc'),
    status: z.enum(['active', 'inactive', 'all']).default('all')
  }));

  // params.page, params.pageSize are numbers (coerced from strings)

  return response.json({ success: true, params });
});
```

## Zod Schema Examples

```typescript
const z = await createZod();

// String
z.string()
z.string().min(1)
z.string().max(100)
z.string().email()
z.string().url()
z.string().regex(/^[a-z]+$/)

// Number
z.number()
z.coerce.number()  // Convert string to number
z.number().min(0)
z.number().max(100)
z.number().int()   // Integer only

// Boolean
z.boolean()
z.coerce.boolean()  // Convert string to boolean

// Enum
z.enum(['admin', 'user', 'guest'])

// Optional
z.string().optional()

// Default
z.string().default('guest')
z.coerce.number().default(1)

// Arrays
z.array(z.string())
z.array(z.number()).min(1).max(10)

// Objects
z.object({
  name: z.string(),
  age: z.number(),
  email: z.string().email()
})

// Nested
z.object({
  user: z.object({
    id: z.number(),
    name: z.string()
  })
})

// Union
z.union([z.string(), z.number()])
z.discriminator('type', {
  admin: z.object({ type: z.literal('admin'), permissions: z.array(z.string()) }),
  user: z.object({ type: z.literal('user'), level: z.number() })
})
```

## formatZodError - Format Validation Errors

```typescript
export default Page(async function(ctx, { body, formatZodError, response }) {
  try {
    const data = body(z.object({
      name: z.string().min(2),
      email: z.string().email()
    }));
    return response.json({ success: true });
  } catch (error) {
    // Format error for user display
    const formatted = formatZodError(error);
    // Returns:
    // {
    //   success: false,
    //   message: 'Validation failed',
    //   errors: [
    //     { path: ['name'], message: 'String must contain at least 2 character(s)', code: 'too_small' }
    //   ]
    // }
    return response.json(formatted, 400);
  }
});
```

## Best Practices

- Always validate request input with `body()` for POST/PUT
- Always validate query parameters with `query()` for GET
- Use `z.coerce.number()` for parameters that might be strings
- Use `z.coerce.boolean()` for boolean flags
- Use `formatZodError` to return user-friendly error messages
- Define schemas at the top of your handler for clarity
