// Check local destinations and GitHub-style anchors without contacting websites.
import { stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";

export type MarkdownLinkProblem = { file: string; target: string; reason: string };

function decodeEntities(text: string): string {
  const named: Record<string, string> = {
    amp: "&",
    lt: "<",
    gt: ">",
    quot: '"',
    apos: "'",
    nbsp: " ",
  };
  return text
    .replace(/&#(x[\da-f]+|\d+);/gi, (_, value: string) =>
      String.fromCodePoint(
        value.toLowerCase().startsWith("x") ? parseInt(value.slice(1), 16) : Number(value),
      ),
    )
    .replace(/&(amp|lt|gt|quot|apos|nbsp);/g, (_, name: string) => named[name]!);
}

function anchors(html: string): Set<string> {
  const result = new Set<string>();
  for (const match of html.matchAll(/<h[1-6](?:\s[^>]*)?>([\s\S]*?)<\/h[1-6]>/g)) {
    const text = decodeEntities(match[1]!.replace(/<[^>]*>/g, ""));
    const base = text
      .toLowerCase()
      .replace(/[^\p{L}\p{M}\p{N}\p{Pc}\s-]/gu, "")
      .replace(/\s/g, "-");
    let slug = base;
    for (let suffix = 1; result.has(slug); suffix++) slug = `${base}-${suffix}`;
    result.add(slug);
  }
  for (const match of html.matchAll(/\b(?:id|name)=["']([^"']+)["']/g))
    result.add(decodeEntities(match[1]!));
  return result;
}

export function markdownFiles(root: string): string[] {
  const result = Bun.spawnSync(
    ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "*.md"],
    { cwd: root },
  );
  if (result.exitCode) throw new Error(result.stderr.toString());
  return result.stdout.toString().split("\0").filter(Boolean);
}

export async function checkMarkdownLinks(root: string, files = markdownFiles(root)) {
  const problems: MarkdownLinkProblem[] = [];
  const pages = new Map<string, string>();
  const headingCache = new Map<string, Set<string>>();
  const page = async (file: string) => {
    if (!pages.has(file)) pages.set(file, Bun.markdown.html(await Bun.file(file).text()));
    return pages.get(file)!;
  };
  let localLinks = 0;
  for (const file of files) {
    const source = resolve(root, file);
    // Parsing Markdown first handles reference links and skips code examples.
    for (const match of (await page(source)).matchAll(/\b(?:href|src)="([^"]*)"/g)) {
      const target = decodeEntities(match[1]!);
      if (!target || /^[a-z][\w+.-]*:|^\/\//i.test(target)) continue;
      localLinks++;
      const [path, fragment] = target.split("#", 2);
      let destination: string;
      let anchor: string;
      try {
        const filename = decodeURIComponent(path!.split("?")[0]!);
        destination = filename
          ? resolve(filename.startsWith("/") ? root : dirname(source), filename.replace(/^\//, ""))
          : source;
        anchor = decodeURIComponent(fragment ?? "").replace(/^user-content-/, "");
      } catch {
        problems.push({ file, target, reason: "invalid URL encoding" });
        continue;
      }
      let info;
      try {
        info = await stat(destination);
      } catch {
        problems.push({ file, target, reason: "missing file or directory" });
        continue;
      }
      if (info.isDirectory()) {
        for (const name of ["README.md", "readme.md"]) {
          if (await Bun.file(resolve(destination, name)).exists()) {
            destination = resolve(destination, name);
            break;
          }
        }
      }
      if (anchor && /\.md$/i.test(destination)) {
        if (!headingCache.has(destination))
          headingCache.set(destination, anchors(await page(destination)));
        if (!headingCache.get(destination)!.has(anchor))
          problems.push({ file, target, reason: "missing heading or HTML anchor" });
      }
    }
  }
  return { files: files.length, localLinks, problems };
}

if (import.meta.main) {
  const result = await checkMarkdownLinks(resolve(import.meta.dir, ".."));
  for (const problem of result.problems)
    console.error(`${problem.file}: ${problem.target} (${problem.reason})`);
  console.log(
    `Checked ${result.localLinks} local references in ${result.files} Markdown files; ${result.problems.length} broken.`,
  );
  if (result.problems.length) process.exitCode = 1;
}
