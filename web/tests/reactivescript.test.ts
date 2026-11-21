import { afterAll, expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";

// Fixtures stay beneath web so package and JSX resolution match the real project.
const cache = new URL("../.cache/", import.meta.url).pathname;
await Bun.write(join(cache, ".keep"), "");
const directory = await mkdtemp(join(cache, "rx-check-"));
afterAll(() => rm(directory, { recursive: true, force: true }));

async function check(source: string) {
  await Bun.write(join(directory, "App.rx"), source);
  await Bun.write(
    join(directory, "tsconfig.json"),
    JSON.stringify({
      extends: "../../tsconfig.json",
      include: ["App.rx"],
    }),
  );
  const child = Bun.spawn(
    [
      process.execPath,
      "--no-env-file",
      new URL("../scripts/typecheck.ts", import.meta.url).pathname,
      join(directory, "tsconfig.json"),
    ],
    { stdout: "pipe", stderr: "pipe" },
  );
  const [status, output, errors] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  return { status, diagnostics: output + errors };
}

test("website checking accepts sources and equations", async () => {
  const result = await check(`export default component App() {
  source count := 0
  doubled = count * 2
  <button onClick={() => { count := count + 1 }}>{doubled}</button>
}`);
  expect(result.status, result.diagnostics).toBe(0);
});

test("website checking rejects writes to derived values", async () => {
  const result = await check(`export default component App() {
  source count := 0
  doubled = count * 2
  <button onClick={() => { doubled := 1 }}>{doubled}</button>
}`);
  expect(result.status).not.toBe(0);
  expect(result.diagnostics).toContain("WRITE_TO_NON_SOURCE");
});

test("website checking enforces props across rx modules", async () => {
  await Bun.write(
    join(directory, "Child.rx"),
    `export component Child(props: { value: number }) {
  <p>{props.value}</p>
}`,
  );
  const result = await check(`import { Child } from "./Child.rx"
export default component App() {
  <Child value={"wrong type"} />
}`);
  expect(result.status).not.toBe(0);
  expect(result.diagnostics).toContain("not assignable to type 'number'");
});

test("native refs capture typed elements through named actions", async () => {
  const result = await check(`export default component App() {
  source video: HTMLVideoElement | undefined := undefined
  action captureVideo(element: HTMLVideoElement) { video := element }
  <video ref={captureVideo} />
  <button disabled={!video}>Play</button>
}`);
  expect(result.status, result.diagnostics).toBe(0);
});

test("native refs reject a callback for the wrong element type", async () => {
  const result = await check(`export default component App() {
  source element: HTMLDivElement | undefined := undefined
  action captureDiv(value: HTMLDivElement) { element := value }
  <video ref={captureDiv} aria-label={element ? "attached" : "missing"} />
}`);
  expect(result.status).not.toBe(0);
  expect(result.diagnostics).toContain("HTMLVideoElement");
});
