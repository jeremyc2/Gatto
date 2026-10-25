import { describe, expect, test } from "bun:test";
import worker from "../cloudflare/worker";

const asset = new TextEncoder().encode("0123456789");
function assetResponse() {
  // Match the deployed failure: the asset binding ignores Range and sends all bytes.
  return new Response(asset, {
    headers: { "Content-Type": "video/mp4", "Content-Length": "10", ETag: '"video"' },
  });
}

async function requestVideo(range?: string, options: { method?: string; ifRange?: string } = {}) {
  const headers = new Headers();
  if (range) headers.set("Range", range);
  if (options.ifRange) headers.set("If-Range", options.ifRange);
  const request = new Request("https://example.com/assets/story-journey-test.mp4", {
    method: options.method ?? "GET",
    headers,
  });
  return worker.fetch(request, { ASSETS: { fetch: async () => assetResponse() } });
}

describe("deployed video byte ranges", () => {
  test.each([
    ["bytes=0-1", "01", "bytes 0-1/10"],
    ["bytes=4-6", "456", "bytes 4-6/10"],
    ["bytes=7-", "789", "bytes 7-9/10"],
    ["bytes=-3", "789", "bytes 7-9/10"],
    ["bytes=7-99", "789", "bytes 7-9/10"],
    ["bytes=-99", "0123456789", "bytes 0-9/10"],
  ])(
    "serves %s even when the static asset binding ignores Range",
    async (range, body, contentRange) => {
      const response = await requestVideo(range);
      expect(response.status).toBe(206);
      expect(response.headers.get("Content-Range")).toBe(contentRange);
      expect(response.headers.get("Content-Length")).toBe(String(body.length));
      expect(response.headers.get("Accept-Ranges")).toBe("bytes");
      expect(response.headers.get("Content-Type")).toBe("video/mp4");
      expect(response.headers.get("ETag")).toBe('"video"');
      expect(await response.text()).toBe(body);
    },
  );

  test.each(["bytes=10-", "bytes=8-2", "bytes=-0"])("rejects unsatisfiable %s", async (range) => {
    const response = await requestVideo(range);
    expect(response.status).toBe(416);
    expect(response.headers.get("Content-Range")).toBe("bytes */10");
    expect(await response.text()).toBe("");
  });

  test.each([undefined, "bytes=bad", "bytes=0-1,4-5", "bytes=-"])(
    "preserves full responses for absent or unsupported ranges: %s",
    async (range) => {
      const response = await requestVideo(range);
      expect(response.status).toBe(200);
      expect(await response.text()).toBe("0123456789");
    },
  );

  test("only applies a range when If-Range matches the current strong validator", async () => {
    expect((await requestVideo("bytes=0-1", { ifRange: '"video"' })).status).toBe(206);
    expect((await requestVideo("bytes=0-1", { ifRange: '"old"' })).status).toBe(200);
    expect((await requestVideo("bytes=0-1", { ifRange: 'W/"video"' })).status).toBe(200);
  });

  test("preserves missing assets and does not turn HEAD into a partial GET", async () => {
    const request = new Request("https://example.com/assets/missing.mp4", {
      headers: { Range: "bytes=0-1" },
    });
    const missing = await worker.fetch(request, {
      ASSETS: { fetch: async () => new Response("Missing", { status: 404 }) },
    });
    expect(missing.status).toBe(404);
    const head = await requestVideo("bytes=0-1", { method: "HEAD" });
    expect(head.status).toBe(200);
    expect(head.headers.get("Content-Length")).toBe("10");
    expect(head.headers.get("Accept-Ranges")).toBe("bytes");
    expect(await head.text()).toBe("");
  });
});
