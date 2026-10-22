/** Fail closed on new findings; one reviewed, expiring lint-only exception. */
type PackageDependencies = {
  dependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
};
export type AuditLock = {
  packages: Record<string, [string, string, PackageDependencies, string]>;
};
export type AuditManifest = {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
};
const advisory = "https://github.com/advisories/GHSA-vfj7-8cjw-p6xm";
const expiry = "2026-11-22T00:00:00Z";
const expected = {
  "@stylexjs/eslint-plugin": ["0.19.1", "sha512-875KpR4M71F5rTSTfDoGWbs09u8atZvVGWBN5bWt3svIEmuNwTMRZGdY8iHiaPrL+W49/tifc6pouKcn3t8d+A=="],
  micromatch: ["4.0.8", "sha512-PXwfBhYu0hBCPw8Dn0E+WDYb7af3dSLVWKi3HGv84IdF4TyFoC0ysxFd0Goxw7nSv4T/PzEJQxsYsEiFCKo2BA=="],
  braces: ["3.0.3", "sha512-yQbXgO/OSZVD2IsiLlro+7Hf6Q18EJrKSEsdoMzKePKXct3gvD8oLcOQdIzGupr5Fj+EDe8gO/lxc1BzfMpxvA=="],
} satisfies Record<string, [string, string]>;

function isDependencies(value: unknown): value is PackageDependencies {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  return ["dependencies", "optionalDependencies", "peerDependencies"].every((key) => {
    if (!(key in value)) return true;
    const record = Reflect.get(value, key);
    return record && typeof record === "object" && !Array.isArray(record) && Object.values(record).every((item) => typeof item === "string");
  });
}

export function parseAuditLock(source: string): AuditLock {
  const value = Bun.JSONC.parse(source);
  if (!value || typeof value !== "object" || !("packages" in value) || !value.packages || typeof value.packages !== "object") throw new Error("Invalid Bun lockfile");
  const result: AuditLock = { packages: Object.create(null) };
  for (const [name, entry] of Object.entries(value.packages)) {
    if (!Array.isArray(entry) || entry.length < 4) continue; // Local workspace entry.
    if (typeof entry[0] !== "string" || typeof entry[1] !== "string" || !isDependencies(entry[2]) || typeof entry[3] !== "string") throw new Error(`Invalid locked package ${name}`);
    result.packages[name] = [entry[0], entry[1], entry[2], entry[3]];
  }
  return result;
}

function reviewedPath(lock: AuditLock, manifests: AuditManifest[]): boolean {
  for (const [name, [version, integrity]] of Object.entries(expected)) {
    const entry = lock.packages[name];
    if (!entry || entry[0] !== `${name}@${version}` || entry[1] !== "" || entry[3] !== integrity) return false;
    if (Object.entries(lock.packages).some(([key, value]) => key !== name && value[0].startsWith(`${name}@`))) return false;
  }
  const roots = new Set(manifests.flatMap((manifest) => Object.keys({ ...manifest.dependencies, ...manifest.devDependencies })));
  if (roots.has("braces") || roots.has("micromatch") || manifests.some((manifest) => manifest.dependencies?.["@stylexjs/eslint-plugin"])) return false;
  const parents = new Map<string, Set<string>>();
  const seen = new Set<string>();
  function resolve(from: string, name: string): string | undefined {
    const parts = from.split("/");
    while (parts.length) {
      const key = `${parts.join("/")}/${name}`;
      if (lock.packages[key]) return key;
      parts.pop();
      if (parts.at(-1)?.startsWith("@")) parts.pop();
    }
    return lock.packages[name] ? name : undefined;
  }
  function visit(key: string) {
    if (seen.has(key)) return;
    seen.add(key);
    const entry = lock.packages[key];
    if (!entry) throw new Error(`Missing locked package ${key}`);
    for (const name of Object.keys({ ...entry[2].dependencies, ...entry[2].optionalDependencies, ...entry[2].peerDependencies })) {
      const dependency = resolve(key, name);
      if (!dependency) continue; // Uninstalled optional/peer packages aren't a route.
      const incoming = parents.get(dependency) ?? new Set<string>();
      incoming.add(key);
      parents.set(dependency, incoming);
      visit(dependency);
    }
  }
  for (const root of roots) visit(root);
  const onlyParent = (name: string, parent: string) => parents.get(name)?.size === 1 && parents.get(name)?.has(parent);
  return roots.has("@stylexjs/eslint-plugin") && !parents.has("@stylexjs/eslint-plugin") &&
    onlyParent("micromatch", "@stylexjs/eslint-plugin") === true && onlyParent("braces", "micromatch") === true;
}

export function checkAudit(report: unknown, exitCode: number, lock: AuditLock, manifests: AuditManifest[], now = new Date()) {
  if (!report || typeof report !== "object" || Array.isArray(report) || ![0, 1].includes(exitCode)) throw new Error("Dependency audit did not return a valid registry report");
  const entries = Object.entries(report);
  if ((entries.length === 0) !== (exitCode === 0)) throw new Error("Dependency audit exit status disagrees with its registry report");
  let accepted = false;
  const problems: string[] = [];
  for (const [name, findings] of entries) {
    if (!Array.isArray(findings) || findings.length === 0) throw new Error("Malformed dependency advisory list");
    for (const finding of findings) {
      if (!finding || typeof finding !== "object" || typeof finding.url !== "string" || typeof finding.title !== "string" || typeof finding.severity !== "string") throw new Error("Malformed dependency advisory");
      if (name === "braces" && finding.url === advisory && finding.severity === "high" && finding.vulnerable_versions === "<=3.0.3" && now.getTime() < Date.parse(expiry) && reviewedPath(lock, manifests)) accepted = true;
      else problems.push(`${name}: ${finding.title} (${finding.url})`);
    }
  }
  if (problems.length) throw new Error(`Dependency audit requires review:\n${problems.join("\n")}`);
  return accepted ? `Reviewed lint-only advisory remains: ${advisory}\nOwner: Jeremy (@jeremyc2). Expires ${expiry}. See SECURITY.md.` : "No known JavaScript dependency vulnerabilities found.";
}

if (import.meta.main) {
  try {
    const child = Bun.spawn([process.execPath, "--no-env-file", "audit", "--json"], { cwd: new URL("..", import.meta.url).pathname, stdout: "pipe", stderr: "pipe", timeout: 60_000 });
    const [stdout, stderr, status] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
    if (stderr.trim()) console.error(stderr.trim());
    const lock = parseAuditLock(await Bun.file(new URL("../bun.lock", import.meta.url)).text());
    const manifests: AuditManifest[] = await Promise.all(["../package.json", "../web/package.json"].map((path) => Bun.file(new URL(path, import.meta.url)).json()));
    console.log(checkAudit(JSON.parse(stdout), status, lock, manifests));
  } catch (error) {
    console.error(error instanceof Error ? error.message : "Could not complete the dependency audit");
    process.exitCode = 1;
  }
}
