/** Exercise real deployed byte serving; local static previews support ranges automatically. */
export async function checkStoryVideo(site: URL) {
  const htmlResponse = await fetch(site, { cache: "no-store" });
  if (!htmlResponse.ok) throw new Error(`Page returned ${htmlResponse.status}.`);
  const html = await htmlResponse.text();
  const path = /src="([^"]+\.mp4)"/.exec(html)?.[1];
  if (!path) throw new Error("The page has no story video source.");
  const video = new URL(path, site);
  let total: number | undefined;
  for (const range of ["bytes=0-1", "bytes=1000000-1000015", "bytes=-16"]) {
    const response = await fetch(video, { cache: "no-store", headers: { Range: range } });
    const contentRange = response.headers.get("Content-Range");
    if (response.status !== 206 || !contentRange) {
      await response.body?.cancel();
      throw new Error(`${range}: expected 206 with Content-Range, received ${response.status}.`);
    }
    const parsed = /^bytes (\d+)-(\d+)\/(\d+)$/.exec(contentRange);
    if (!parsed) throw new Error(`${range}: malformed Content-Range: ${contentRange}.`);
    const size = Number(parsed[3]);
    total ??= size;
    const expectedStart = range === "bytes=0-1" ? 0 : range === "bytes=-16" ? size - 16 : 1_000_000;
    const expectedLength = range === "bytes=0-1" ? 2 : 16;
    if (
      size !== total ||
      Number(parsed[1]) !== expectedStart ||
      Number(parsed[2]) !== expectedStart + expectedLength - 1 ||
      response.headers.get("Content-Type")?.split(";")[0] !== "video/mp4"
    ) {
      await response.body?.cancel();
      throw new Error(`${range}: unexpected range or video metadata: ${contentRange}.`);
    }
    const bytes = await response.bytes();
    if (bytes.length !== expectedLength) {
      throw new Error(`${range}: expected ${expectedLength} bytes, received ${bytes.length}.`);
    }
    console.log(`${range}: 206 ${contentRange} (${bytes.length} bytes)`);
  }
}

if (import.meta.main) {
  const site = Bun.argv[2];
  if (!site) throw new Error("Pass the deployed website URL.");
  await checkStoryVideo(new URL(site));
}
