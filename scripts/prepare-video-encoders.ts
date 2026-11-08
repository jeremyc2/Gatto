// Build file-only, self-contained macOS helpers. No downloads happen at app runtime.
import { cp, mkdir, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import sources from "./video-encoder-sources.json";

const root = resolve(import.meta.dir, "..");
const cache = join(root, ".tools/video-encoders");
const arch = process.arch === "arm64" ? "arm64" : "x86_64";
const destination = join(cache, arch);
const prefix = join(destination, "static");
const work = join(destination, "build");
const log = join(destination, "build.log");
const environment = {
  PATH: "/usr/bin:/bin:/usr/sbin:/sbin",
  HOME: Bun.env.HOME ?? "",
  MACOSX_DEPLOYMENT_TARGET: "13.0",
  PKG_CONFIG_PATH: join(prefix, "lib/pkgconfig"),
  PKG_CONFIG_LIBDIR: join(prefix, "lib/pkgconfig"),
};

async function run(args: string[], cwd = root): Promise<string> {
  const child = Bun.spawn(args, { cwd, env: environment, stdout: "pipe", stderr: "pipe" });
  const [exit, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  const output = stdout + stderr;
  await Bun.write(
    log,
    (await Bun.file(log)
      .text()
      .catch(() => "")) + output,
  );
  if (exit !== 0)
    throw new Error(`${args[0]} failed (${exit}); see ${log}\n${output.slice(-3000)}`);
  return stdout;
}

async function download(name: keyof typeof sources): Promise<string> {
  const source = sources[name];
  const archive = join(cache, "downloads", `${name}-${source.sha256}.tar`);
  const digest = async (path: string) =>
    new Bun.CryptoHasher("sha256").update(await Bun.file(path).arrayBuffer()).digest("hex");
  if (!(await Bun.file(archive).exists())) {
    const temporary = `${archive}.download`;
    await run([
      "/usr/bin/curl",
      "--fail",
      "--location",
      "--retry",
      "2",
      "--proto",
      "=https",
      "--tlsv1.2",
      source.url,
      "--output",
      temporary,
    ]);
    if ((await digest(temporary)) !== source.sha256) {
      await rm(temporary, { force: true });
      throw new Error(`Checksum mismatch: ${name}`);
    }
    await cp(temporary, archive);
    await rm(temporary);
  }
  if ((await digest(archive)) !== source.sha256) throw new Error(`Corrupt cached source: ${name}`);
  return archive;
}

async function extract(name: keyof typeof sources): Promise<string> {
  const directory = join(work, name);
  await mkdir(directory, { recursive: true });
  const args = ["/usr/bin/tar", "-xf", await download(name), "-C", directory];
  args.push("--strip-components=1");
  await run(args);
  return directory;
}

async function verify(): Promise<void> {
  for (const name of ["ffmpeg"]) {
    const file = join(destination, "bin", name);
    const libraries = await run(["/usr/bin/otool", "-L", file]);
    for (const line of libraries.trim().split("\n").slice(1)) {
      const dependency = line.trim().split(" ")[0]!;
      if (!dependency.startsWith("/usr/lib/") && !dependency.startsWith("/System/Library/"))
        throw new Error(`Non-system dependency in ${name}: ${dependency}`);
    }
    const architectures = (await run(["/usr/bin/lipo", "-archs", file])).trim();
    if (architectures !== arch) throw new Error(`Wrong architecture: ${architectures}`);
    await run([file, "-version"]);
  }
  const codecs = await run([join(destination, "bin/ffmpeg"), "-hide_banner", "-encoders"]);
  for (const name of ["libx264", "libsvtav1", "aac", "wrapped_avframe"])
    if (!codecs.includes(name)) throw new Error(`Missing encoder: ${name}`);
}

async function prepare(): Promise<void> {
  if (process.platform !== "darwin")
    throw new Error("Video encoders require macOS and Xcode command-line tools");
  await mkdir(join(cache, "downloads"), { recursive: true });
  await mkdir(destination, { recursive: true });
  const fingerprint = new Bun.CryptoHasher("sha256")
    .update(await Bun.file(import.meta.path).arrayBuffer())
    .update(JSON.stringify(sources))
    .update(await Bun.file(join(root, "packaging/video-inspect.c")).arrayBuffer())
    .digest("hex");
  const marker = join(destination, "fingerprint");
  if (
    (await Bun.file(marker)
      .text()
      .catch(() => "")) !== fingerprint
  ) {
    await rm(work, { recursive: true, force: true });
    await rm(prefix, { recursive: true, force: true });
    await rm(join(destination, "bin"), { recursive: true, force: true });
    await rm(join(destination, "video-encoder-sources.tar.xz"), { force: true });
    await Bun.write(log, "");
    const tools = join(work, "tools");
    const cmake = join(await extract("cmake"), "CMake.app/Contents/bin/cmake");
    const pkgconf = await extract("pkgconf");
    const jobs = "4";
    console.log("Building codec tools (progress in .tools/video-encoders/" + arch + "/build.log)…");
    await run(["./configure", `--prefix=${tools}`, "--disable-shared", "--enable-static"], pkgconf);
    await run(["/usr/bin/make", "-j", jobs, "install"], pkgconf);
    const x264 = await extract("x264");
    await run(
      [
        "./configure",
        `--prefix=${prefix}`,
        "--enable-static",
        "--disable-cli",
        "--disable-opencl",
        "--bit-depth=8",
        ...(arch === "x86_64" ? ["--disable-asm"] : []),
      ],
      x264,
    );
    await run(["/usr/bin/make", "-j", jobs, "install"], x264);
    const cmakeFlags = [
      "-G",
      "Unix Makefiles",
      `-DCMAKE_INSTALL_PREFIX=${prefix}`,
      "-DCMAKE_BUILD_TYPE=Release",
      "-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0",
      `-DCMAKE_OSX_ARCHITECTURES=${arch}`,
      "-DBUILD_SHARED_LIBS=OFF",
    ];
    for (const name of ["svt-av1", "aom"] as const) {
      console.log(`Building ${name}…`);
      const source = await extract(name);
      const build = join(work, `${name}-build`);
      const options =
        name === "svt-av1"
          ? [
              "-DBUILD_APPS=OFF",
              "-DBUILD_TESTING=OFF",
              ...(arch === "x86_64" ? ["-DENABLE_NASM=OFF"] : []),
            ]
          : [
              "-DCONFIG_AV1_ENCODER=0",
              "-DCONFIG_AV1_DECODER=1",
              "-DENABLE_EXAMPLES=OFF",
              "-DENABLE_TOOLS=OFF",
              "-DENABLE_TESTS=OFF",
              "-DENABLE_DOCS=OFF",
              ...(arch === "x86_64" ? ["-DAOM_TARGET_CPU=generic"] : []),
            ];
      await run([cmake, "-S", source, "-B", build, ...cmakeFlags, ...options]);
      await run([cmake, "--build", build, "--parallel", jobs]);
      await run([cmake, "--install", build]);
    }
    console.log("Building FFmpeg…");
    const ffmpeg = await extract("ffmpeg");
    await cp(join(root, "packaging/video-inspect.c"), join(ffmpeg, "fftools/gatto-inspect.c"));
    const entry = join(ffmpeg, "fftools/ffmpeg.c");
    const original = await Bun.file(entry).text();
    const main = "int main(int argc, char **argv)\n{";
    if (original.split(main).length !== 2)
      throw new Error("FFmpeg entry point changed; review inspection integration");
    await Bun.write(
      entry,
      original.replace(
        main,
        '#include "gatto-inspect.c"\n\n' +
          main +
          '\n    if (argc == 3 && !strcmp(argv[1], "--gatto-inspect")) return gatto_inspect(argv[2]);',
      ),
    );
    await run(
      [
        "./configure",
        `--prefix=${destination}`,
        `--pkg-config=${tools}/bin/pkgconf`,
        "--pkg-config-flags=--static",
        `--extra-cflags=-I${prefix}/include -mmacosx-version-min=13.0`,
        `--extra-ldflags=-L${prefix}/lib -mmacosx-version-min=13.0`,
        "--extra-libs=-lc++",
        "--enable-gpl",
        "--enable-version3",
        "--disable-shared",
        "--enable-static",
        "--disable-autodetect",
        "--disable-everything",
        "--disable-network",
        "--disable-doc",
        "--disable-debug",
        "--enable-small",
        "--enable-ffmpeg",
        "--disable-ffprobe",
        "--enable-protocol=file,pipe",
        "--enable-demuxer=mov,matroska",
        "--enable-muxer=mp4,null",
        "--enable-parser=h264,hevc,vp8,vp9,av1,aac,mpegaudio,opus,vorbis",
        "--enable-decoder=h264,hevc,vp8,vp9,prores,mpeg4,aac,opus,vorbis,mp3,pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_f64le",
        "--enable-libaom",
        "--enable-decoder=libaom_av1",
        "--enable-libx264",
        "--enable-libsvtav1",
        "--enable-encoder=libx264,libsvtav1,aac,wrapped_avframe",
        "--enable-filter=fps,scale,format,aformat,aresample,null,anull",
        "--enable-swscale",
        "--enable-swresample",
        ...(arch === "x86_64" ? ["--disable-x86asm"] : []),
      ],
      ffmpeg,
    );
    await run(["/usr/bin/make", "-j", jobs], ffmpeg);
    await run(["/usr/bin/make", "install"], ffmpeg);
    await verify();
    const notices = join(destination, "notices");
    await mkdir(notices, { recursive: true });
    for (const [name, paths] of Object.entries({
      ffmpeg: ["COPYING.GPLv3", "LICENSE.md"],
      x264: ["COPYING"],
      "svt-av1": ["LICENSE.md", "PATENTS.md"],
      aom: ["LICENSE", "PATENTS"],
    })) {
      for (const path of paths) await cp(join(work, name, path), join(notices, `${name}-${path}`));
    }
    await cp(join(root, "packaging/video-encoders-NOTICE"), join(notices, "NOTICE"));
    await Bun.write(marker, fingerprint);
  } else await verify();
  // Accompany every distributed helper with exact source archives and its build recipe.
  const archive = join(destination, "video-encoder-sources.tar.xz");
  if (!(await Bun.file(archive).exists()) || Bun.argv.includes("--archive")) {
    const staging = join(work, "corresponding-source");
    await rm(staging, { recursive: true, force: true });
    await mkdir(join(staging, "scripts"), { recursive: true });
    await mkdir(join(staging, "packaging"), { recursive: true });
    for (const name of ["ffmpeg", "x264", "svt-av1", "aom", "pkgconf", "cmake"] as const)
      await cp(await download(name), join(staging, `${name}.tar`));
    for (const name of ["prepare-video-encoders.ts", "video-encoder-sources.json"])
      await cp(join(root, "scripts", name), join(staging, "scripts", name));
    await cp(
      join(root, "packaging/video-encoders-NOTICE"),
      join(staging, "packaging/video-encoders-NOTICE"),
    );
    await cp(join(root, "packaging/video-inspect.c"), join(staging, "packaging/video-inspect.c"));
    await cp(join(destination, "notices"), join(staging, "notices"), { recursive: true });
    await Bun.write(
      join(staging, "BUILDING.md"),
      "# Video encoder source\n\nInstall Bun and Xcode command-line tools on macOS 13+. Copy each NAME.tar to .tools/video-encoders/downloads/NAME-SHA256.tar, using the hashes in scripts/video-encoder-sources.json. Run `bun scripts/prepare-video-encoders.ts` from this directory. The recipe checks every archive, builds static libraries, and verifies that the executables use only system dynamic libraries. It requires no network access when the six verified archives are cached.\n",
    );
    await run(["/usr/bin/tar", "-cJf", archive, "-C", staging, "."]);
  }
  console.log(`Video encoders ready: ${destination}`);
}

if (import.meta.main) await prepare();
