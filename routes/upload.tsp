// Slice 16g: request.formData() with multipart/form-data.
// Demonstrates that binary bodies (raw bytes) reach Bun's
// native Request formData parser without U+FFFD corruption.
// File parts via `multipart/form-data` are an open Bun
// question (`Request.formData()` on a Blob-fed body hits
// a parser hang in Bun 1.4); we keep the test focused on
// text fields + raw-body-byte round-trip, which is what
// the slice-16g "raw body transport" promise is really
// about. File upload with full byte-fidelity is a future
// slice once the body transport is upgraded to streaming
// (plan sect.14.3 file-upload DoD).
export async function POST(ctx) {
  try {
    const fd = await ctx.request.formData();
    const lines = [];
    for (const [name, value] of fd.entries()) {
      if (typeof value === 'string') {
        lines.push(`${name}=${value}`);
      } else {
        lines.push(`${name}=file(${value.size} bytes, type=${value.type})`);
      }
    }
    return new Response(lines.sort().join('; '), {
      status: 200,
      headers: { "x-demo": "slice16g", "content-type": "text/plain" },
    });
  } catch (e) {
    // formData() can fail when the body is not a parseable
    // multipart payload (e.g. plain text). Surface the
    // error in the response so the E2E shows it instead
    // of hanging.
    return new Response(`formData-error: ${e}`, {
      status: 500,
      headers: { "x-demo": "slice16g", "content-type": "text/plain" },
    });
  }
}

export function GET() {
  return "POST a multipart/form-data body to /upload to see formData() parsed\n";
}
