import { expect, test } from "bun:test";
import { updateCask } from "./update-homebrew-cask";

const source = await Bun.file(new URL("../Casks/gatto.rb", import.meta.url)).text();
const digest = "a".repeat(64);
test("cask follows the exact stable release asset and checksum", () => {
  const sums = `${"b".repeat(64)}  appcast.xml\n${digest}  gatto-0.15.0-aarch64-apple-darwin.dmg\n`;
  const result = updateCask(source, "0.15.0", sums);
  expect(result).toContain('version "0.15.0"');
  expect(result).toContain(`sha256 "${digest}"`);
  expect(result).toContain("auto_updates true");
  expect(updateCask(result, "0.15.0", sums)).toBe(result);
});
test("rejects a missing, duplicated, wrong-architecture or malformed checksum", () => {
  const entry = `${digest}  gatto-0.15.0-aarch64-apple-darwin.dmg\n`;
  for (const sums of [
    "",
    entry + entry,
    `${digest}  gatto-0.15.0-x86_64-apple-darwin.dmg`,
    "oops  gatto-0.15.0-aarch64-apple-darwin.dmg",
  ]) {
    expect(() => updateCask(source, "0.15.0", sums)).toThrow();
  }
  expect(() => updateCask(source, "0.15.0-beta.1", entry)).toThrow();
});
