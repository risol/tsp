
export default async function (_context: PageContext) {
  // 直接返回 Response 对象，确保重定向工作
  return new Response(null, {
    status: 302,
    headers: {
      "Location": "/"
    }
  });
}
