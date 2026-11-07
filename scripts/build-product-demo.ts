// Native Gatto interaction frames, composed and rendered headlessly in EffectCraft.
import { link, mkdir, unlink } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const output = resolve(root, process.argv[2] ?? "dist/product-demo");
const editor = resolve(output, "native/editor");
const workflowDirectory = resolve(output, "native/workflow");
const media = resolve(output, "media");
const captures = resolve(media, "frames");
const previews = resolve(output, "previews");
const ec = Bun.env.EFFECTCRAFT_CLI ?? resolve(Bun.env.HOME ?? "", ".local/bin/effectcraft-cli");
if (!Bun.which(ec))
  throw new Error("Install EffectCraft CLI or set EFFECTCRAFT_CLI before capturing.");
const fps = 30;
type Point = [number, number];
type CursorKind = "pointer" | "crosshair" | "ibeam";
type CaptureFrame = {
  frame: number;
  time: number;
  file: string;
  cursor: Point;
  pressed: boolean;
  cursorKind: CursorKind;
  event: string | null;
};
type Captures = {
  width: number;
  height: number;
  fps: number;
  frames: number;
  circleStart: Point;
  circleEnd: Point;
  arrowStart: Point;
  arrowEnd: Point;
  circleTool: Point;
  textAnchor: Point;
  saveButton: Point;
  timeline: CaptureFrame[];
};
type State = {
  id: string;
  file: string;
  cursor: Point;
  duration: number;
  click: boolean;
  focus: Point;
};
type Workflow = { width: number; height: number; states: State[]; outro: State[] };
type Camera = { center: Point; scale: number };
type FilmFrame = {
  file: string;
  cursor: Point;
  cursorKind: CursorKind;
  camera: Camera;
  event: string | null;
  scene: string;
  drag: boolean;
};
type SoundEvent = {
  cue: "tool" | "draw" | "place" | "camera" | "type";
  time: number;
  duration?: number;
  gain?: number;
};
type Step = { command: string; params: Record<string, unknown> };

async function run(command: string[], quiet = false) {
  const child = Bun.spawn(command, {
    cwd: root,
    stdout: quiet ? "pipe" : "inherit",
    stderr: "inherit",
  });
  const result = quiet ? await new Response(child.stdout).text() : "";
  if (await child.exited) throw new Error(`Demo generation failed: ${command[0]}\n${result}`);
  return result;
}

for (const folder of [captures, media, previews, editor, workflowDirectory])
  await mkdir(folder, { recursive: true });
if (!process.argv.includes("--reuse-captures")) {
  const command = [
    "cargo",
    "run",
    "--offline",
    "--locked",
    "--features",
    "walkthrough-screenshots",
    "--bin",
    "walkthrough-screenshots",
    "--",
  ];
  await run([...command, editor, "--annotation-demo"]);
  await run([
    ...command,
    workflowDirectory,
    "--product-demo",
    "--demo-annotation",
    resolve(editor, "final-annotated.png"),
  ]);
}
const native = (await Bun.file(resolve(editor, "metadata.json")).json()) as Captures;
const workflow = (await Bun.file(resolve(workflowDirectory, "workflow.json")).json()) as Workflow;
if (native.fps !== fps || native.frames !== fps * 14 || native.timeline.length !== 420)
  throw new Error("Native editor capture must contain 420 frames at 30 fps.");
if (native.width !== 2000 || native.height !== 1520)
  throw new Error("The camera layout requires 2000 × 1520 native captures at 2× backing scale.");
if (workflow.width !== native.width || workflow.height !== native.height || !workflow.outro.length)
  throw new Error("The project workflow needs matching native dimensions and a saved-image outro.");
if (
  !["upload-complete", "copy-confirmed"].every((id) =>
    workflow.outro.some((state) => state.id === id),
  )
)
  throw new Error("Recapture the workflow ending to include upload and clipboard confirmation.");

function ease(value: number) {
  const t = Math.max(0, Math.min(1, value));
  return t * t * t * (t * (t * 6 - 15) + 10);
}
const blend = (a: number, b: number, t: number) => a + (b - a) * t;
const mix = (a: Point, b: Point, t: number): Point => [blend(a[0], b[0], t), blend(a[1], b[1], t)];
const wide: Camera = { center: [1000, 760], scale: 0.6 };
const medium: Camera = { center: [1000, 900], scale: 0.88 };
const between = (a: Camera, b: Camera, t: number): Camera => ({
  center: mix(a.center, b.center, ease(t)),
  scale: blend(a.scale, b.scale, ease(t)),
});
const film: FilmFrame[] = [];
const sounds: SoundEvent[] = [];
const chapters: { id: string; frame: number }[] = [];
const dragOrigin: Point = [1870, 420];
const holdDurations: Record<string, number> = {
  empty: 1,
  "create-dialog": 0.65,
  "name-ready": 0.4,
  "project-created": 0.7,
  "project-menu": 1.35,
  "project-menu-dismissed": 0.85,
  "image-staged": 0.75,
  "attachment-menu": 1.35,
  "annotate-choice": 0.35,
  "project-annotated": 0.6,
  "upload-ready": 0.6,
  "upload-progress-00": 0.35,
  "upload-progress-35": 0.45,
  "upload-progress-70": 0.45,
  "upload-complete": 0.85,
  "copy-ready": 0.45,
  "copy-confirmed": 1.4,
  "final-clean": 2,
};
function workflowCamera(id: string): Camera {
  if (id === "empty") return wide;
  if (id === "create-dialog" || id.startsWith("name-") || id === "confirm-create")
    return { center: [1000, 520], scale: 1.05 };
  if (id === "project-created") return { center: [1000, 650], scale: 0.78 };
  if (id === "project-menu") return { center: [850, 500], scale: 1.04 };
  if (id === "attachment-menu" || id === "annotate-choice")
    return { center: [1300, 700], scale: 1.02 };
  if (id === "project-annotated") return { center: [1120, 900], scale: 0.82 };
  if (id === "upload-ready" || id === "upload-progress-00")
    return { center: [1450, 1240], scale: 1.08 };
  if (id.startsWith("upload-progress-")) return { center: [1250, 570], scale: 1.02 };
  if (id === "upload-complete") return { center: [1250, 500], scale: 0.96 };
  if (id.startsWith("copy-")) return { center: [1250, 500], scale: 1.02 };
  if (id === "final-clean") return wide;
  return { center: [1100, 620], scale: 0.82 };
}
function appendWorkflow(states: State[], outro = false) {
  for (const [index, state] of states.entries()) {
    const count = state.id.match(/^name-\d/)
      ? 3
      : Math.round((holdDurations[state.id] ?? state.duration) * fps);
    const next = states[index + 1];
    const previous = film.at(-1)?.camera ?? wide;
    const target = workflowCamera(state.id);
    chapters.push({ id: state.id, frame: film.length });
    if (state.click)
      sounds.push({
        cue: state.id === "image-staged" || state.id === "copy-confirmed" ? "place" : "tool",
        time: film.length / fps,
        gain: state.id === "image-staged" ? 0.75 : state.id === "copy-confirmed" ? 0.5 : 0.6,
      });
    if (state.id === "upload-complete")
      sounds.push({ cue: "place", time: film.length / fps, gain: 0.45 });
    if (state.id.match(/^name-\d/))
      sounds.push({ cue: "type", time: film.length / fps, gain: 0.7 });
    for (let j = 0; j < count; j++) {
      const drag = state.id === "project-menu-dismissed";
      const origin = outro && index === 0 ? native.saveButton : state.cursor;
      let cursor = origin;
      if (outro && state.id === "final-clean") cursor = film.at(-1)?.cursor ?? origin;
      if (drag) cursor = mix(dragOrigin, state.cursor, ease(j / Math.max(1, count - 1)));
      else if (next && !(outro && next.id === "final-clean")) {
        const destination = next.id === "project-menu-dismissed" ? dragOrigin : next.cursor;
        const travelFrames = Math.min(count, outro ? 15 : 9);
        const travel = state.id === "upload-complete" ? j : j - count + travelFrames;
        cursor = mix(origin, destination, ease(travel / Math.max(1, travelFrames - 1)));
      }
      const camera = between(previous, target, j / (state.id === "final-clean" ? 48 : 13));
      film.push({
        file: resolve(workflowDirectory, state.file),
        cursor,
        cursorKind: "pointer",
        camera,
        event: state.click && j === 0 ? "click" : null,
        scene: state.id,
        drag,
      });
    }
  }
}
appendWorkflow(workflow.states);
const editorStart = film.length;
const previousShot = film.at(-1)!;
const enteringCursor: Point = [
  1000 +
    ((previousShot.cursor[0] - previousShot.camera.center[0]) * previousShot.camera.scale) /
      medium.scale,
  900 +
    ((previousShot.cursor[1] - previousShot.camera.center[1]) * previousShot.camera.scale) /
      medium.scale,
];
chapters.push({ id: "editor", frame: editorStart });
function editorCamera(time: number): Camera {
  const circle: Camera = { center: [720, 580], scale: 1.32 };
  const arrow: Camera = { center: [790, 600], scale: 1.26 };
  const note: Camera = { center: [820, 570], scale: 1.25 };
  const save: Camera = { center: [1050, 650], scale: 0.78 };
  if (time < 0.65) return medium;
  if (time < 1) return between(medium, circle, (time - 0.65) / 0.35);
  if (time < 2.1) return circle;
  if (time < 3.1) return between(circle, arrow, time - 2.1);
  if (time < 4.6) return arrow;
  if (time < 5.6) return between(arrow, note, time - 4.6);
  if (time < 10.2) return note;
  if (time < 11.2) return between(note, save, time - 10.2);
  return save;
}
for (const source of native.timeline.filter((frame) => frame.frame < 348)) {
  const time = source.frame / fps;
  film.push({
    file: resolve(editor, source.file),
    cursor:
      source.frame < 15
        ? mix(enteringCursor, native.circleTool, ease(source.frame / 15))
        : source.cursor,
    cursorKind: source.cursorKind,
    camera: editorCamera(time),
    event: ["tool", "draw-down", "place", "text-down", "save"].includes(source.event ?? "")
      ? "click"
      : null,
    scene: "editor",
    drag: false,
  });
  const at = (editorStart + source.frame) / fps;
  if (source.event === "tool") sounds.push({ cue: "tool", time: at, gain: 0.6 });
  if (source.event === "place" || source.event === "save")
    sounds.push({ cue: "place", time: at, gain: 0.85 });
  if (source.event === "type" || source.event === "shortcut")
    sounds.push({ cue: "type", time: at, gain: 0.65 });
}
for (const [start, length] of [
  [1, 1.1],
  [3.1, 1.5],
] as const)
  sounds.push({ cue: "draw", time: editorStart / fps + start, duration: length, gain: 0.75 });
const outroStart = film.length;
appendWorkflow(workflow.outro, true);
const duration = film.length / fps;
const closingStart = chapters.find((chapter) => chapter.id === "final-clean")!.frame;
for await (const name of new Bun.Glob("frame-????.png").scan(captures)) {
  if (/^frame-\d{4}\.png$/.test(name)) await unlink(resolve(captures, name));
}
for (const [index, frame] of film.entries()) {
  const path = resolve(captures, `frame-${String(index).padStart(4, "0")}.png`);
  await link(frame.file, path);
}
const audioTimeline = resolve(output, "sound-timeline.json");
await Bun.write(audioTimeline, JSON.stringify({ duration, events: sounds }));
await run([
  "bun",
  "--no-env-file",
  "scripts/generate-demo-audio.ts",
  resolve(media, "audio"),
  audioTimeline,
]);
await new Bun.Image(Bun.file(resolve(root, "docs/images/walkthrough/source/canvas.png")))
  .resize(200, 111)
  .png()
  .write(resolve(media, "dragged-image.png"));
await Bun.write(
  resolve(output, "chapters.json"),
  JSON.stringify({ fps, duration, editorStart, outroStart, chapters }, null, 2),
);

const svg = (width: number, height: number, contents: string) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">${contents}</svg>`;
await Bun.write(
  resolve(media, "studio.svg"),
  svg(
    1920,
    1080,
    `
  <defs>
    <linearGradient id="paper" x2="1" y2="1"><stop stop-color="#f7f3ed"/><stop offset="1" stop-color="#eee9e6"/></linearGradient>
    <radialGradient id="light"><stop stop-color="#d1c5f4" stop-opacity=".42"/><stop offset="1" stop-color="#d1c5f4" stop-opacity="0"/></radialGradient>
  </defs>
  <path fill="url(#paper)" d="M0 0H1920V1080H0Z"/>
  <ellipse cx="1470" cy="230" rx="1050" ry="850" fill="url(#light)"/>`,
  ),
);
await Bun.write(
  resolve(media, "pointer.svg"),
  svg(
    64,
    64,
    `
  <path d="M7 5L7 42L17 33L25 51L33 47L25 30L40 30Z" fill="#ffffff" stroke="#171822" stroke-width="3" stroke-linejoin="round"/>`,
  ),
);
await Bun.write(
  resolve(media, "crosshair.svg"),
  svg(
    64,
    64,
    `
  <path d="M32 10V25M32 39V54M10 32H25M39 32H54" stroke="#171822" stroke-width="5" stroke-linecap="round"/>
  <path d="M32 10V25M32 39V54M10 32H25M39 32H54" stroke="white" stroke-width="2" stroke-linecap="round"/>
  <circle cx="32" cy="32" r="2.5" fill="white" stroke="#171822"/>`,
  ),
);
await Bun.write(
  resolve(media, "press.svg"),
  svg(
    100,
    100,
    `
  <circle cx="50" cy="50" r="40" fill="#8b7bff" fill-opacity=".12" stroke="#a69bff" stroke-width="3"/>`,
  ),
);
await Bun.write(
  resolve(media, "ibeam.svg"),
  svg(
    64,
    64,
    `
  <path d="M23 10H41M32 10V54M23 54H41" fill="none" stroke="#171822" stroke-width="5" stroke-linecap="round"/>
  <path d="M23 10H41M32 10V54M23 54H41" fill="none" stroke="white" stroke-width="2" stroke-linecap="round"/>`,
  ),
);

const steps: Step[] = [];
const add = (command: string, params: Record<string, unknown>) => steps.push({ command, params });
const set = (layer: string, path: string, value: unknown) =>
  add("prop.set", { layer, path, value });
const key = (layer: string, path: string, frame: number, value: unknown) =>
  add("prop.addKey", { layer, path, time: frame / fps, timeBase: "comp", value });
const importAsset = (filename: string, name: string) => {
  add("file.import", { paths: [resolve(media, filename)], addToComp: true });
  add("layer.settings", { layer: filename, name });
};
add("comp.new", {
  name: "Gatto — Palette review",
  width: 1920,
  height: 1080,
  frameRate: fps,
  duration,
  background: "#f4efe9",
});
importAsset("studio.svg", "Studio backdrop");
add("file.import", {
  paths: [resolve(captures, "frame-0000.png")],
  sequence: true,
  frameRate: fps,
  addToComp: true,
});
add("layer.settings", {
  layer: `frame-[0000-${String(film.length - 1).padStart(4, "0")}].png`,
  name: "Native Gatto interaction",
});
add("layer.addMask", {
  layer: "Native Gatto interaction",
  shape: "rounded",
  rect: [0, 0, native.width, native.height],
  roundness: 32,
});
add("layer.style.dropShadow", { layers: ["Native Gatto interaction"] });
set("Native Gatto interaction", "layerStyles/dropShadow/opacity", 24);
set("Native Gatto interaction", "layerStyles/dropShadow/distance", 24);
set("Native Gatto interaction", "layerStyles/dropShadow/size", 65);
importAsset("dragged-image.png", "Dragged image");
add("layer.addMask", {
  layer: "Dragged image",
  shape: "rounded",
  rect: [0, 0, 200, 111],
  roundness: 10,
});
add("layer.style.dropShadow", { layers: ["Dragged image"] });
set("Dragged image", "layerStyles/dropShadow/opacity", 35);
set("Dragged image", "layerStyles/dropShadow/distance", 8);
set("Dragged image", "layerStyles/dropShadow/size", 14);
importAsset("press.svg", "Interaction pulse");
importAsset("pointer.svg", "Pointer");
set("Pointer", "transform/anchor", [7, 5, 0]);
set("Pointer", "transform/scale", [66, 66, 100]);
importAsset("crosshair.svg", "Drawing cursor");
set("Drawing cursor", "transform/anchor", [32, 32, 0]);
set("Drawing cursor", "transform/scale", [55, 55, 100]);
importAsset("ibeam.svg", "Text cursor");
set("Text cursor", "transform/anchor", [32, 32, 0]);
set("Text cursor", "transform/scale", [55, 55, 100]);
add("file.import", { paths: [resolve(media, "audio/mix.wav")], addToComp: true });
add("layer.settings", { layer: "mix.wav", name: "Original canvas Foley" });

const cameras = [];
let lastClick = -Infinity;
for (const [index, frame] of film.entries()) {
  const time = index / fps;
  const cam = frame.camera;
  if (frame.event === "click") lastClick = time;
  const screen = (p: Point): number[] => [
    960 + (p[0] - cam.center[0]) * cam.scale,
    540 + (p[1] - cam.center[1]) * cam.scale,
    0,
  ];
  key(
    "Native Gatto interaction",
    "transform/position",
    index,
    screen([native.width / 2, native.height / 2]),
  );
  key("Native Gatto interaction", "transform/scale", index, [
    cam.scale * 100,
    cam.scale * 100,
    100,
  ]);
  for (const name of ["Pointer", "Drawing cursor", "Text cursor", "Interaction pulse"])
    key(name, "transform/position", index, screen(frame.cursor));
  let cursorFade = 1 - ease((index - closingStart) / 12);
  const editorTime = (index - editorStart) / fps;
  if (frame.scene === "editor" && editorTime >= 8.2 && editorTime < 10.2)
    cursorFade *= 1 - ease((editorTime - 8.2) / 0.25);
  if (frame.scene === "editor" && editorTime >= 10.2 && editorTime < 10.4)
    cursorFade *= ease((editorTime - 10.2) / 0.2);
  for (const [name, kind] of [
    ["Pointer", "pointer"],
    ["Drawing cursor", "crosshair"],
    ["Text cursor", "ibeam"],
  ])
    key(name!, "transform/opacity", index, frame.cursorKind === kind ? 100 * cursorFade : 0);
  const dragged = screen(frame.cursor);
  key("Dragged image", "transform/position", index, [dragged[0]! + 92, dragged[1]! + 52, 0]);
  key("Dragged image", "transform/opacity", index, frame.drag ? 92 : 0);
  const sinceClick = time - lastClick;
  const pulse = sinceClick >= 0 && sinceClick < 0.32;
  key(
    "Interaction pulse",
    "transform/opacity",
    index,
    pulse ? 42 * (1 - sinceClick / 0.32) ** 2 : 0,
  );
  const ringScale = pulse ? 36 + sinceClick * 140 : 36;
  key("Interaction pulse", "transform/scale", index, [ringScale, ringScale, 100]);
  cameras.push({
    frame: index,
    scene: frame.scene,
    cursor: frame.cursor,
    cursorKind: frame.cursorKind,
    ...cam,
  });
}
await Bun.write(resolve(output, "camera.json"), JSON.stringify(cameras, null, 2));
const commands = resolve(output, "effectcraft-commands.json");
await Bun.write(commands, JSON.stringify(steps));
const project = resolve(output, "gatto-product-demo.ecproj");
await run([ec, "run", "--empty", "--commands", commands, "--save-as", project, "--json"], true);
// EffectCraft saves absolute footage paths, but resolves relative paths against
// the project folder. Keep this bundle movable, including every sequence frame.
const prefix = JSON.stringify(`${output}/`).slice(1, -1);
await Bun.write(project, (await Bun.file(project).text()).replaceAll(prefix, ""));
const missing = JSON.parse(
  await run([ec, "exec", "footage.missing", "--project", project, "--json"], true),
) as { missing: unknown[] };
if (missing.missing.length) throw new Error("The EffectCraft bundle has missing media.");
await run([ec, "info", "--project", project, "--json"], true);
for await (const name of new Bun.Glob("*.png").scan(previews)) {
  if (/^\d+\.\d{2}\.png$/.test(name)) await unlink(resolve(previews, name));
}
const previewFrames = [
  0,
  ...chapters
    .filter((chapter) =>
      [
        "create-dialog",
        "name-ready",
        "project-menu",
        "project-menu-dismissed",
        "image-staged",
        "attachment-menu",
        "project-annotated",
        "upload-ready",
        "upload-progress-35",
        "upload-complete",
        "copy-ready",
        "copy-confirmed",
      ].includes(chapter.id),
    )
    .map((chapter) => chapter.frame + 15),
  editorStart + 45,
  editorStart + 115,
  editorStart + 225,
  editorStart + 270,
  film.length - 15,
];
for (const frame of previewFrames) {
  const time = frame / fps;
  await run(
    [
      ec,
      "render-frame",
      "--project",
      project,
      "--comp",
      "Gatto — Palette review",
      "--time",
      String(time),
      "--max-side",
      "1280",
      "--out",
      resolve(previews, `${time.toFixed(2)}.png`),
      "--json",
    ],
    true,
  );
}
await run(
  [
    ec,
    "render",
    "--project",
    project,
    "--comp",
    "Gatto — Palette review",
    "--out",
    resolve(output, "gatto-product-demo.mp4"),
    "--format",
    "h264",
    "--audio",
    "on",
    "--fps",
    "30",
    "--bitrate",
    "16000",
    "--progress",
    "--json",
  ],
  true,
);
console.log(`Editable EffectCraft project and 1080p film: ${output}`);
