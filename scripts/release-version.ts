// Gatto publishes stable x.y.z and beta x.y.z-beta.N versions only.
const pattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-beta\.([1-9]\d*))?$/;
export function versionParts(version: string): [number, number, number, number | undefined] {
  const match = pattern.exec(version);
  if (!match) throw new Error("Expected x.y.z or x.y.z-beta.N (N starts at 1)");
  const values = match.slice(1).map((value) => (value === undefined ? undefined : Number(value)));
  if (values.some((value) => value !== undefined && !Number.isSafeInteger(value)))
    throw new Error("Version component is too large");
  if (values[3] !== undefined && values[3] > 255)
    throw new Error("Beta number must be between 1 and 255 for macOS bundle versions");
  return [values[0]!, values[1]!, values[2]!, values[3]];
}
export function releaseChannel(version: string): "stable" | "beta" {
  return versionParts(version)[3] === undefined ? "stable" : "beta";
}
export function compareVersions(a: string, b: string): number {
  const left = versionParts(a),
    right = versionParts(b);
  for (let i = 0; i < 3; i++) {
    const difference = left[i]! - right[i]!;
    if (difference) return Math.sign(difference);
  }
  if (left[3] === right[3]) return 0;
  if (left[3] === undefined) return 1;
  if (right[3] === undefined) return -1;
  return Math.sign(left[3] - right[3]);
}
export function nextVersion(previous: string, choice: string): string {
  const [major, minor, patch, beta] = versionParts(previous);
  if ((choice === "stable" || choice === "promote") && beta === undefined)
    throw new Error("Only a beta release can be promoted to Stable");
  const result =
    choice === "major"
      ? `${major + 1}.0.0`
      : choice === "minor"
        ? `${major}.${minor + 1}.0`
        : choice === "patch"
          ? `${major}.${minor}.${patch + 1}`
          : choice === "stable" || choice === "promote"
            ? `${major}.${minor}.${patch}`
            : choice === "beta"
              ? beta === undefined
                ? `${major}.${minor + 1}.0-beta.1`
                : `${major}.${minor}.${patch}-beta.${beta + 1}`
              : choice;
  if (compareVersions(result, previous) <= 0)
    throw new Error("Release version must be newer than the published release");
  return result;
}

export type ReleaseOption = { key: string; label: string; version: string };

/** Show destinations before asking the user to choose a release operation. */
export function releaseOptions(
  current: string,
  stable: string,
  latest: string,
  prepared: boolean,
): ReleaseOption[] {
  const base = compareVersions(current, latest) > 0 ? current : latest;
  const options: ReleaseOption[] = [];
  const add = (key: string, label: string, version: string) => {
    const published = releaseChannel(version) === "beta" ? latest : stable;
    if (compareVersions(version, current) >= 0 && compareVersions(version, published) > 0)
      options.push({ key, label, version });
  };
  if (prepared) add("prepared", "Release the prepared version", current);
  if (releaseChannel(base) === "beta") {
    // Apple's beta suffix ends at 255; promotion and custom versions stay usable.
    if (versionParts(base)[3]! < 255)
      add("beta", "Advance to the next Beta", nextVersion(base, "beta"));
    add("stable", "Promote this Beta to Stable", nextVersion(base, "stable"));
  } else {
    add("beta", "Start a Beta for the next minor release", nextVersion(base, "beta"));
  }
  for (const key of ["patch", "minor", "major"])
    add(key, `New ${key} release (Stable)`, nextVersion(base, key));
  return options;
}

export function selectReleaseVersion(
  options: ReleaseOption[],
  choice: string,
  current: string,
  stable: string,
  latest: string,
): string {
  const input = choice.trim().toLowerCase();
  const selected = /^\d+$/.test(input)
    ? options[Number(input) - 1]
    : options.find((option) => option.key === (input === "promote" ? "stable" : input));
  const version = selected?.version ?? input;
  const published = releaseChannel(version) === "beta" ? latest : stable;
  if (compareVersions(version, published) <= 0)
    throw new Error("Release version must be newer than the published channel");
  if (compareVersions(version, current) < 0)
    throw new Error("Cannot downgrade the version already prepared in Cargo.toml");
  return version;
}

// Sparkle treats a hyphen as a build suffix and ignores it in comparisons.
// Apple's bN bundle suffix preserves beta ordering and stable promotion.
export function bundleVersion(version: string): string {
  versionParts(version);
  return version.replace("-beta.", "b");
}
