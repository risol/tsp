// 02-login.tsp -- form POST handler.
// Maps to:  GET, POST  /login
//
// Demonstrates: freeze 4 (multiple method exports on the same file),
// freeze 6 (ctx.request.formData, ctx.services), freeze 5
// (redirect helper), freeze 9 (form values are escaped in the
// echoed <p>).

import {
  type Context,
  type PageConfig,
  redirect,
  text,
} from "tsp:server";

export const config = {
  auth: "none",
  bodyLimit: 16 * 1024, // 16 KiB is plenty for a username + password
  cache: "no-store",
} satisfies PageConfig;

export function GET(_ctx: Context) {
  return (
    <html>
      <head><title>Sign in</title></head>
      <body>
        <h1>Sign in</h1>
        <form method="post">
          <input name="username" required />
          <input name="password" type="password" required />
          <button type="submit">Sign in</button>
        </form>
      </body>
    </html>
  );
}

export async function POST(ctx: Context) {
  const form = await ctx.request.formData();
  const username = String(form.get("username") ?? "").trim();
  const password = String(form.get("password") ?? "");

  if (!username || !password) {
    return text("username and password are required", { status: 400 });
  }

  const ok = await ctx.services.users.verifyCredentials(username, password);
  if (!ok) {
    return text("invalid credentials", { status: 401 });
  }

  await ctx.session.set("user", { username });
  return redirect("/dashboard");
}
