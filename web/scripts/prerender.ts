/** Vite owns both bundles. Bun turns the generated root response into a static page. */
const { handleRequest } = (await import(
  new URL("../dist/server/server.js", import.meta.url).href
)) as {
  handleRequest: (request: Request) => Promise<Response>;
};
// Localhost is only a base for the page URL; the handler renders it in memory.
const prerenderUrl = new URL("/", "http://localhost");
const response = await handleRequest(new Request(prerenderUrl));
if (!response.ok) throw new Error(`Prerender failed: ${response.status}`);
await Bun.write(new URL("../dist/client/index.html", import.meta.url), await response.text());
console.log("Prerendered Gatto → web/dist/client/");
