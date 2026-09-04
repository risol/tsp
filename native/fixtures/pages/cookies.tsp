export function GET(ctx) {
  const seen = ctx.cookies.get("sid") || "none";
  ctx.cookies.set("sid", "native-session", { path: "/", httpOnly: true });
  ctx.cookies.set("theme", "dark", { path: "/", maxAge: 3600 });
  return `<p>seen=${seen}</p><p>has=${ctx.cookies.has("sid")}</p>`;
}
