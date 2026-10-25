import { beforeAll, expect, test } from "bun:test";
import { cssSize, staticDirectory } from "../scripts/css-size";

beforeAll(async () => {
  const process = Bun.spawn(["bun", "--no-env-file", "run", "build:web"], {
    cwd: new URL("../../", import.meta.url).pathname,
    env: Object.fromEntries([
      ...Object.entries(Bun.env).filter(
        (entry): entry is [string, string] => typeof entry[1] === "string",
      ),
      ["NODE_ENV", "production"],
    ]),
    stdout: "pipe",
    stderr: "pipe",
  });
  const [status, output, errors] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ]);
  if (status !== 0) throw new Error(`Website build failed\n${output}\n${errors}`);
}, 30_000);

test("prerenders all scenes, concise copy and the two README download badges", async () => {
  const html = await Bun.file(new URL("index.html", staticDirectory)).text();
  expect(html).toContain("Gatto — GitHub, illustrated.");
  expect(html).toContain("Choose a repository, then drop in your files.");
  expect(html).toContain("Add several files at once.");
  expect(html).toContain("Annotate before uploading.");
  expect(html).toContain("Compare two images.");
  expect(html.match(/id="chapter-(?:bring|upload|copy)"/g)).toHaveLength(3);
  expect(html.match(/alt="Download Gatto for macOS"/g)).toHaveLength(2);
  const preloadedFonts = [
    ...html.matchAll(/<link[^>]+rel="preload"[^>]+href="([^"]+)"[^>]+as="font"/g),
  ];
  expect(preloadedFonts).toHaveLength(2);
  expect(html.match(/(?:src|href)="[^"]+\.webp"/g)).toHaveLength(5);
  expect(html).not.toContain("Still view");
  expect(html).not.toContain("How it works");
  const articles = html.match(/<article\b[^>]*id="chapter-[^"]+"[^>]*>/g) ?? [];
  expect(articles).toHaveLength(3);
  for (const article of articles) {
    expect(article).not.toContain('aria-hidden="true"');
    expect(article).not.toContain("inert");
  }
  const files = new Set(
    [...html.matchAll(/(?:src|href)="([^"#]+\.(?:js|css|png|webp|svg|woff2))"/g)]
      .map((match) => match[1])
      .filter((path): path is string => Boolean(path && !path.startsWith("https:"))),
  );
  for (const path of files)
    expect(await Bun.file(new URL(path.replace(/^\//, ""), staticDirectory)).exists()).toBe(true);
});

test("ships one extracted stylesheet within the CSS budget, with themes and reduced motion", async () => {
  const size = await cssSize();
  expect(size.files).toHaveLength(1);
  expect(size.bytes).toBeLessThan(23 * 1024);
  expect(size.gzip).toBeLessThan(7 * 1024);
  const path = size.files[0]?.path;
  if (!path) throw new Error("The extracted stylesheet is missing.");
  const css = await Bun.file(new URL(path, staticDirectory)).text();
  expect(css).toContain("@layer");
  expect(css).toContain("prefers-reduced-motion");
  expect(css).toContain("prefers-color-scheme");
  expect(css).not.toContain("data:font");
});

test("ships one seekable video within its transfer budget, without fetching it before hydration", async () => {
  const html = await Bun.file(new URL("index.html", staticDirectory)).text();
  const videos = html.match(/<video\b[^>]*>/g) ?? [];
  expect(videos).toHaveLength(1);
  expect(videos[0]).not.toMatch(/\ssrc=|\sautoplay/);
  const paths = Array.from(new Bun.Glob("assets/*.mp4").scanSync(staticDirectory.pathname));
  expect(paths).toHaveLength(1);
  const bytes = await Bun.file(new URL(paths[0]!, staticDirectory)).bytes();
  expect(bytes.length).toBeLessThan(8 * 1024 * 1024);
  const buffer = Buffer.from(bytes);
  expect(buffer.indexOf("moov")).toBeGreaterThan(0);
  expect(buffer.indexOf("moov")).toBeLessThan(buffer.indexOf("mdat"));
});
