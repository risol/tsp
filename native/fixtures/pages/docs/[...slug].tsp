export function GET(ctx) {
  return `catch-all:${ctx.params.slug}`;
}
