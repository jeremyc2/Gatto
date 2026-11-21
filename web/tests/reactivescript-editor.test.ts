import { expect, test } from "bun:test";

test("the editor language server initializes with the repository-root TypeScript SDK", async () => {
  const root = new URL("../../", import.meta.url);
  const server = new URL(
    "../node_modules/@reactivescript/language-server/bin/server.js",
    import.meta.url,
  );
  const child = Bun.spawn([process.execPath, "--no-env-file", server.pathname, "--stdio"], {
    stdin: "pipe",
    stdout: "pipe",
    stderr: "pipe",
    timeout: 5_000,
  });
  const errors = new Response(child.stderr).text();
  const request = JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: {
      processId: process.pid,
      rootUri: root.href,
      workspaceFolders: [{ uri: root.href, name: "Gatto" }],
      capabilities: {},
      initializationOptions: {
        typescript: { tsdk: new URL("node_modules/typescript/lib", root).pathname },
      },
    },
  });
  let pending = Buffer.alloc(0);
  try {
    await child.stdin.write(`Content-Length: ${Buffer.byteLength(request)}\r\n\r\n${request}`);
    await child.stdin.flush();
    for await (const chunk of child.stdout) {
      pending = Buffer.concat([pending, chunk]);
      while (true) {
        const headerEnd = pending.indexOf("\r\n\r\n");
        if (headerEnd < 0) break;
        const header = pending.subarray(0, headerEnd).toString();
        const match = /Content-Length:\s*(\d+)/i.exec(header);
        if (!match) throw new Error(`Invalid language-server response: ${header}`);
        const bodyStart = headerEnd + 4;
        const bodyEnd = bodyStart + Number(match[1]);
        if (pending.length < bodyEnd) break;
        const response = JSON.parse(pending.subarray(bodyStart, bodyEnd).toString());
        pending = pending.subarray(bodyEnd);
        if (response.id !== 1) continue;
        expect(response.error, JSON.stringify(response.error)).toBeUndefined();
        expect(response.result.capabilities).toBeObject();
        return;
      }
    }
    throw new Error(`Language server exited before initialization: ${await errors}`);
  } finally {
    child.kill();
    await child.exited;
    await errors;
  }
}, 10_000);
