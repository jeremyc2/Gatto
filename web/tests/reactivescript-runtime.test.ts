import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";
import { app } from "@reactivescript/compiler";
import { renderToString, type JSX } from "@solidjs/web";
import { createSiteRuntime } from "../src/reactivescript";

test("compiled children preserve text arrays, escape once and omit null", async () => {
  const cache = new URL("../.cache/", import.meta.url).pathname;
  await Bun.write(join(cache, ".keep"), "");
  const directory = await mkdtemp(join(cache, "rx-runtime-"));
  try {
    const source = join(directory, "App.rx");
    await Bun.write(
      source,
      `component Caption(props: { children: import("@solidjs/web").JSX.Element }) {
  <p>{props.children}</p>
}
export default component App() {
  <Caption>First line
    second line</Caption>
  <Caption>{"<tag> & quoted"}</Caption>
  <Caption>{null}</Caption>
}`,
    );
    const compiled = JSON.parse(app(source, "ssr"));
    const code: unknown = compiled.modules?.[source]?.code;
    if (typeof code !== "string") throw new Error(JSON.stringify(compiled));
    const output = join(directory, "compiled.mjs");
    await Bun.write(output, code);
    const { default: program } = (await import(output)) as {
      default: (runtime: ReturnType<typeof createSiteRuntime>) => () => JSX.Element;
    };
    const html = renderToString(program(createSiteRuntime())).replace(/<!--.*?-->/gs, "");
    expect(html).toMatch(/<p[^>]*>First line second line<\/p>/);
    expect(html).toContain("&lt;tag> &amp; quoted");
    expect(html).toMatch(/<p[^>]*><\/p>/);
    expect(html).not.toContain("&amp;lt;");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
