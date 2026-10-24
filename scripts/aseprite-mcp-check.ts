import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

type Result = {
  protocolVersion?: string;
  instructions?: string;
  tools?: { name: string }[];
  content?: { type: string; text?: string; data?: string; mimeType?: string }[];
  isError?: boolean;
};

type Message = {
  id?: number;
  result?: Result;
  error?: { message: string };
};

function text(result: Result): string {
  const value = result.content?.find((item) => item.type === "text")?.text;
  assert(value, "Tool returned no text content");
  return value;
}

async function checkImage(filename: string, format: string): Promise<void> {
  const image = new Bun.Image(Bun.file(filename));
  const metadata = await image.metadata();
  assert.deepEqual(metadata, { width: 16, height: 16, format });
  // Metadata only reads headers; encoding forces a full decode of the first frame.
  await image.png().bytes();
}

// A sequential MCP stdio client for this check, not a general-purpose MCP SDK.
// https://modelcontextprotocol.io/specification/2024-11-05/basic/transports
export async function checkAseprite(env: Record<string, string | undefined>): Promise<void> {
  const root = resolve(import.meta.dir, "..");
  const temporary = await mkdtemp(join(tmpdir(), "gatto-aseprite-check-"));
  const server = Bun.spawn(
    [process.execPath, "--no-env-file", resolve(root, "scripts/aseprite-mcp.ts"), "--stdio"],
    { cwd: root, env, stdin: "pipe", stdout: "pipe", stderr: "inherit" },
  );
  const reader = server.stdout.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  let id = 0;
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    server.kill("SIGTERM");
  }, 60_000);

  async function request(method: string, params: object = {}): Promise<Result> {
    const requestId = ++id;
    server.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: requestId, method, params }) + "\n");
    await server.stdin.flush();
    while (true) {
      const newline = buffer.indexOf("\n");
      if (newline !== -1) {
        const line = buffer.slice(0, newline);
        buffer = buffer.slice(newline + 1);
        if (!line.trim()) continue;
        const message: Message = JSON.parse(line);
        if (message.id === undefined) continue; // Server notifications have no response ID.
        assert.equal(message.id, requestId, "Unexpected MCP response ID");
        if (message.error) throw new Error(`${method}: ${message.error.message}`);
        assert(message.result, `${method}: missing MCP result`);
        return message.result;
      }
      const chunk = await reader.read();
      if (chunk.done) {
        throw new Error(
          timedOut ? "Aseprite check timed out" : "Aseprite MCP closed before replying",
        );
      }
      buffer += decoder.decode(chunk.value, { stream: true });
    }
  }

  async function call(name: string, args: object): Promise<Result> {
    const result = await request("tools/call", { name, arguments: args });
    assert(!result.isError, `${name}: ${JSON.stringify(result.content)}`);
    // Upstream also reports operation failures as ordinary text results.
    const message = result.content?.find((item) => item.type === "text")?.text ?? "";
    assert(!/^(Failed|Invalid|ERROR:)|not found/i.test(message), `${name}: ${message}`);
    return result;
  }

  try {
    const initialized = await request("initialize", {
      protocolVersion: "2024-11-05",
      capabilities: {},
      clientInfo: { name: "gatto-aseprite-check", version: "1.0.0" },
    });
    assert.equal(initialized.protocolVersion, "2024-11-05");
    assert(initialized.instructions, "Server omitted its pixel-art instructions");
    server.stdin.write(
      JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) + "\n",
    );
    await server.stdin.flush();
    const { tools } = await request("tools/list");
    assert(tools?.length, "Server returned no tools");

    const sprite = join(temporary, "check.aseprite");
    const png = join(temporary, "check.png");
    const gif = join(temporary, "check.gif");
    await call("create_canvas", { width: 8, height: 8, filename: sprite });
    await call("add_layer", { filename: sprite, layer_name: "ink" });
    await call("draw_rows", {
      filename: sprite,
      layer_name: "ink",
      frame_index: 1,
      x: 2,
      y: 2,
      rows: ["RR", "RR"],
      legend: { R: "#FF0000FF" },
    });
    assert(await Bun.file(sprite).exists(), "Native Aseprite file was not created");
    await call("export_frame", {
      filename: sprite,
      frame_index: 1,
      output_filename: png,
      scale: 2,
    });
    await checkImage(png, "png");
    const pixels = JSON.parse(
      text(
        await call("get_composite_rect", {
          filename: png,
          x: 0,
          y: 0,
          width: 5,
          height: 5,
        }),
      ),
    );
    assert(Array.isArray(pixels) && pixels.length === 25, "Exported PNG pixel read failed");
    const red = pixels.find((pixel) => pixel.x === 4 && pixel.y === 4);
    const clear = pixels.find((pixel) => pixel.x === 0 && pixel.y === 0);
    assert(red && clear, "Exported PNG omitted expected pixels");
    assert.deepEqual([red.r, red.g, red.b, red.a], [255, 0, 0, 255]);
    assert.equal(clear.a, 0, "Exported PNG lost transparency");

    const preview = await call("view_sprite", { filename: sprite });
    const picture = preview.content?.find((item) => item.type === "image");
    assert(picture?.data, "view_sprite returned no image content");
    await new Bun.Image(Buffer.from(picture.data, "base64")).png().bytes();
    await call("animation_workflow_guide", { use_case: "idle" });
    await call("add_frame", { filename: sprite });
    await call("draw_rows", {
      filename: sprite,
      layer_name: "ink",
      frame_index: 2,
      x: 3,
      y: 2,
      rows: ["BB", "BB"],
      legend: { B: "#0000FFFF" },
    });
    const source = JSON.parse(text(await call("get_sprite_info", { filename: sprite })));
    assert.equal(source.frames, 2, "Native sprite should contain two frames");
    await call("export_animation", { filename: sprite, output_filename: gif, scale: 2 });
    await checkImage(gif, "gif");
    // Bun decodes a GIF's first frame; reopen it in Aseprite to check the animation.
    const animation = JSON.parse(text(await call("get_sprite_info", { filename: gif })));
    assert.equal(animation.frames, 2, "Exported GIF should contain two frames");
    assert.equal(animation.width, 16);
    assert.equal(animation.height, 16);
    console.log(
      `Aseprite MCP OK: ${tools.length} tools, instructions, native sprite, PNG alpha, image preview, two-frame GIF.`,
    );
  } finally {
    clearTimeout(timeout);
    server.stdin.end();
    const terminate = setTimeout(() => server.kill("SIGTERM"), 1_000);
    const kill = setTimeout(() => server.kill("SIGKILL"), 3_000);
    try {
      await server.exited;
      await reader.cancel();
    } finally {
      clearTimeout(terminate);
      clearTimeout(kill);
      await rm(temporary, { recursive: true, force: true });
    }
  }
}
