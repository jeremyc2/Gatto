import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { parseEnv } from "node:util";
import { storyVideo } from "../src/story-video";

const root = resolve(import.meta.dir, "../..");
const art = resolve(root, "docs/design/explorations/art/scroll-story");
const localEnv = Bun.file(resolve(root, ".env.aseprite"));
const aseprite =
  Bun.env.ASEPRITE_PATH ||
  ((await localEnv.exists()) ? parseEnv(await localEnv.text()).ASEPRITE_PATH : undefined);
if (!aseprite) throw new Error("Set ASEPRITE_PATH in .env.aseprite or your environment.");
for (const command of [aseprite, "ffmpeg", "ffprobe"])
  if (!Bun.which(command)) throw new Error(`Install the artwork export tool: ${command}`);

const environment = Object.fromEntries(
  Object.entries(Bun.env).filter(
    ([name]) => !name.startsWith("CLOUDFLARE_") && !name.startsWith("CF_"),
  ),
);
async function run(command: string[]) {
  const child = Bun.spawn(command, {
    cwd: root,
    env: environment,
    stdout: "inherit",
    stderr: "inherit",
  });
  if ((await child.exited) !== 0) throw new Error(`Artwork export failed: ${command[0]}`);
}

const output = await mkdtemp(resolve(tmpdir(), "gatto-story-"));
try {
  await mkdir(resolve(root, "web/assets/video"), { recursive: true });
  await run([
    aseprite,
    "--batch",
    resolve(art, "journey.aseprite"),
    "--script-param",
    `output=${output}`,
    "--script-param",
    `frames=${storyVideo.frames}`,
    "--script-param",
    `width=${storyVideo.width}`,
    "--script-param",
    `height=${storyVideo.height}`,
    "--script",
    resolve(art, "export-frames.lua"),
  ]);
  await run([
    "ffmpeg",
    "-hide_banner",
    "-loglevel",
    "error",
    "-y",
    "-framerate",
    String(storyVideo.fps),
    "-i",
    resolve(output, "frame-%03d.png"),
    "-frames:v",
    String(storyVideo.frames),
    "-c:v",
    "libx264",
    "-preset",
    "veryslow",
    "-crf",
    "24",
    "-g",
    "12",
    "-keyint_min",
    "12",
    "-sc_threshold",
    "0",
    "-bf",
    "0",
    "-pix_fmt",
    "yuv420p",
    "-an",
    "-movflags",
    "+faststart",
    resolve(root, "web/assets/video/story-journey.mp4"),
  ]);
  // Static readers see discovery, carrying, and the completed resting scene.
  for (const [frame, name] of [
    [1, "bring"],
    [47, "upload"],
    [storyVideo.frames, "copy"],
  ] as const)
    await new Bun.Image(Bun.file(resolve(output, `frame-${String(frame).padStart(3, "0")}.png`)))
      .webp({ lossless: true })
      .write(resolve(root, `web/assets/images/story-${name}.webp`));
  await run([
    "ffprobe",
    "-v",
    "error",
    "-show_entries",
    "format=size,duration:stream=codec_name,width,height,nb_frames,r_frame_rate,pix_fmt",
    "-of",
    "json",
    resolve(root, "web/assets/video/story-journey.mp4"),
  ]);
} finally {
  await rm(output, { recursive: true, force: true });
}
