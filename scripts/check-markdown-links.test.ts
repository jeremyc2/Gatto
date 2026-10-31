import { expect, test } from "bun:test";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { checkMarkdownLinks } from "./check-markdown-links";

test("repository Markdown file, image and heading references resolve", async () => {
  const result = await checkMarkdownLinks(resolve(import.meta.dir, ".."));
  expect(result.problems).toEqual([]);
  expect(result.localLinks).toBeGreaterThan(500);
});

test("checks rendered reference links, duplicate headings, HTML anchors and encoded paths", async () => {
  const root = await mkdtemp(resolve(tmpdir(), "gatto-markdown-"));
  try {
    await mkdir(resolve(root, "guide"));
    await Bun.write(
      resolve(root, "guide/README.md"),
      '# First `heading`!\n\n## First heading!\n\n<a id="explicit"></a>\n',
    );
    await Bun.write(resolve(root, "a picture.png"), "fixture");
    await Bun.write(
      resolve(root, "README.md"),
      `
[Heading][guide]
[Duplicate](guide/#first-heading-1)
[Explicit](guide/README.md#explicit)
![Picture](a%20picture.png)
<a href="guide/README.md#first-heading">HTML link</a>
[Missing](absent.md)
[Bad anchor](guide/README.md#absent)
[Remote](https://example.com/unchecked)
[Invalid encoding](bad%XX.md)

\`[Example](not-a-link.md)\`

\`\`\`md
[Code example](also-not-a-link.md)
\`\`\`

[guide]: guide/README.md#first-heading
`,
    );
    const result = await checkMarkdownLinks(root, ["README.md"]);
    expect(result.localLinks).toBe(8);
    expect(result.problems).toEqual([
      { file: "README.md", target: "absent.md", reason: "missing file or directory" },
      {
        file: "README.md",
        target: "guide/README.md#absent",
        reason: "missing heading or HTML anchor",
      },
      { file: "README.md", target: "bad%XX.md", reason: "invalid URL encoding" },
    ]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
