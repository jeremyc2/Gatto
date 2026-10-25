// Workers Static Assets currently ignores Range, which leaves cold video seeks stuck.
// Only MP4 requests run this Worker; other assets retain direct static serving.
export default {
  async fetch(request: Request, env: { ASSETS: { fetch(request: Request): Promise<Response> } }) {
    const response = await env.ASSETS.fetch(request);
    return videoRangeResponse(request, response);
  },
};

export async function videoRangeResponse(request: Request, response: Response): Promise<Response> {
  if ((request.method !== "GET" && request.method !== "HEAD") || response.status !== 200)
    return response;
  const headers = new Headers(response.headers);
  headers.set("Accept-Ranges", "bytes");
  if (request.method === "HEAD") return new Response(null, { headers });
  const range = /^bytes=(\d*)-(\d*)$/i.exec(request.headers.get("Range")?.trim() ?? "");
  const ifRange = request.headers.get("If-Range");
  if (
    !range ||
    (!range[1] && !range[2]) ||
    (ifRange && (ifRange.startsWith("W/") || ifRange !== response.headers.get("ETag")))
  ) {
    return new Response(response.body, { status: response.status, headers });
  }

  // The build enforces an 8 MiB budget for the single bundled story video.
  // Buffer only this bounded asset, so Workers can infer the exact response length.
  const video = await response.arrayBuffer();
  const size = video.byteLength;
  const start = range[1] ? Number(range[1]) : Math.max(0, size - Number(range[2]));
  const end = range[1] && range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
  if (start > end || start >= size) {
    headers.set("Content-Range", `bytes */${size}`);
    headers.set("Content-Length", "0");
    return new Response(null, { status: 416, headers });
  }
  headers.set("Content-Range", `bytes ${start}-${end}/${size}`);
  headers.set("Content-Length", String(end - start + 1));
  return new Response(video.slice(start, end + 1), { status: 206, headers });
}
