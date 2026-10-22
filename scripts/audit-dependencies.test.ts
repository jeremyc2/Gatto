import { expect, test } from "bun:test";
import { checkAudit, parseAuditLock, type AuditManifest } from "./audit-dependencies";

const current = parseAuditLock(await Bun.file(new URL("../bun.lock", import.meta.url)).text());
const manifests: AuditManifest[] = await Promise.all(["../package.json", "../web/package.json"].map((path) => Bun.file(new URL(path, import.meta.url)).json()));
const report = { braces: [{ url: "https://github.com/advisories/GHSA-vfj7-8cjw-p6xm", title: "Stack exhaustion", severity: "high", vulnerable_versions: "<=3.0.3" }] };
const date = new Date("2026-10-22T00:00:00Z");

test("audit fails closed on invalid reports and registry failures", () => {
  for (const value of [null, [], { braces: [] }, { braces: [null] }]) expect(() => checkAudit(value, 1, current, manifests, date)).toThrow();
  expect(() => checkAudit({}, 1, current, manifests, date)).toThrow();
  expect(() => checkAudit(report, 2, current, manifests, date)).toThrow();
  expect(() => checkAudit(report, 0, current, manifests, date)).toThrow();
  expect(checkAudit({}, 0, current, manifests, date)).toContain("No known");
});

test("only the reviewed current dev-only path receives a visible temporary exception", () => {
  expect(checkAudit(report, 1, current, manifests, date)).toContain("Expires 2026-11-22");
  expect(() => checkAudit(report, 1, current, manifests, new Date("2026-11-22T00:00:00Z"))).toThrow();
  expect(() => checkAudit({ other: report.braces }, 1, current, manifests, date)).toThrow();
  expect(() => checkAudit({ braces: [{ ...report.braces[0], severity: "critical" }] }, 1, current, manifests, date)).toThrow();
  expect(() => checkAudit({ braces: [...report.braces, { ...report.braces[0], url: "https://github.com/advisories/GHSA-new" }] }, 1, current, manifests, date)).toThrow();
  const changed = structuredClone(current);
  const braces = changed.packages.braces;
  if (!braces) throw new Error("Expected the current braces fixture");
  braces[3] = "sha512-changed";
  expect(() => checkAudit(report, 1, changed, manifests, date)).toThrow();
  const production = [...manifests, { dependencies: { "@stylexjs/eslint-plugin": "0.19.1" } }];
  expect(() => checkAudit(report, 1, current, production, date)).toThrow();
});

test("a second route to the vulnerable package invalidates the exception", () => {
  const changed = structuredClone(current);
  changed.packages["other-tool"] = ["other-tool@1.0.0", "", { dependencies: { braces: "3.0.3" } }, "sha512-fixture"];
  expect(() => checkAudit(report, 1, changed, [...manifests, { devDependencies: { "other-tool": "1.0.0" } }], date)).toThrow();
  const nested = structuredClone(current);
  const braces = nested.packages.braces;
  if (!braces) throw new Error("Expected the current braces fixture");
  nested.packages["micromatch/braces"] = structuredClone(braces);
  expect(() => checkAudit(report, 1, nested, manifests, date)).toThrow();
});
