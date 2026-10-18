/** Vite owns both bundles. Bun turns the generated root response into a static page. */
const { handleRequest } = (await import(
  new URL("../dist/server/server.js", import.meta.url).href
)) as {
  handleRequest: (request: Request) => Promise<Response>;
};
// Each path gets a real static document, including direct loads and refreshes.
for (const path of ["/", "/gattoverse/"]) {
  const response = await handleRequest(new Request(new URL(path, "http://localhost")));
  if (!response.ok) throw new Error(`Prerender failed for ${path}: ${response.status}`);
  await Bun.write(
    new URL(`../dist/client${path}index.html`, import.meta.url),
    await response.text(),
  );
}
console.log("Prerendered Gatto → web/dist/client/");
