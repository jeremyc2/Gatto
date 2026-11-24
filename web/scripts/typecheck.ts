/** rx-tsc can log a failed analysis and still exit zero; incomplete checks must fail. */
const checker = new URL(
  "../node_modules/@reactivescript/language-server/bin/tsc.js",
  import.meta.url,
);
const project = Bun.argv[2] ?? new URL("../tsconfig.json", import.meta.url).pathname;
const child = Bun.spawn(
  [process.execPath, "--no-env-file", checker.pathname, "--noEmit", "-p", project],
  {
    stdout: "pipe",
    stderr: "pipe",
  },
);
const [status, output, errors] = await Promise.all([
  child.exited,
  new Response(child.stdout).text(),
  new Response(child.stderr).text(),
]);
if (output) process.stdout.write(output);
if (errors) process.stderr.write(errors);
if (status !== 0 || /rx: analyzing .* failed:/.test(errors)) {
  process.exitCode = status || 1;
}
