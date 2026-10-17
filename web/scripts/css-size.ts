export const staticDirectory = new URL("../dist/client/", import.meta.url);

/** Measure the actual stylesheets shipped by the prerendered page, excluding fonts and images. */
export async function cssSize(directory = staticDirectory) {
  const html = await Bun.file(new URL("index.html", directory)).text();
  const paths = new Set([...html.matchAll(/href="\/?([^" ]+\.css)"/g)].map((match) => match[1]));
  const files = [];
  for (const path of paths) {
    if (!path || path.startsWith("https:")) continue;
    const data = await Bun.file(new URL(path, directory)).bytes();
    files.push({ path, bytes: data.length, gzip: Bun.gzipSync(data).length });
  }
  return {
    files,
    bytes: files.reduce((n, file) => n + file.bytes, 0),
    gzip: files.reduce((n, file) => n + file.gzip, 0),
  };
}

if (import.meta.main) {
  const size = await cssSize();
  console.log(JSON.stringify(size, null, 2));
}
