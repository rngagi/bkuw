import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL("../../", import.meta.url));
const target = process.platform === "darwin" && process.arch === "arm64" ? "aarch64-apple-darwin"
  : process.platform === "win32" && process.arch === "x64" ? "x86_64-pc-windows-msvc" : null;
if (!target) throw new Error("Audio tools support macOS Apple Silicon and Windows x64.");
let ready = false;
try {
  const manifest = JSON.parse(readFileSync(new URL("../../src-tauri/resources/audio/manifest.json", import.meta.url)));
  const suffix = process.platform === "win32" ? ".exe" : "";
  const recipe = createHash("sha256").update(readFileSync(new URL("./prepare.sh", import.meta.url))).digest("hex");
  for (const file of ["FFmpeg-LICENSE.txt", "LAME-LICENSE.txt", "sources/ffmpeg-8.0.1.tar.xz", "sources/lame-3.100.tar.gz", "sources/prepare.sh"]) {
    readFileSync(new URL(`../../src-tauri/resources/audio/${file}`, import.meta.url));
  }
  ready = manifest.target === target && manifest.recipeSha256 === recipe && ["ffmpeg", "ffprobe"].every((tool) => {
    const bytes = readFileSync(new URL(`../../src-tauri/resources/audio/${tool}${suffix}`, import.meta.url));
    return createHash("sha256").update(bytes).digest("hex") === manifest.sha256[`${tool}${suffix}`];
  });
} catch { /* A clean checkout builds its pinned tools once. */ }
if (!ready) {
  const command = process.platform === "win32" ? "msys2" : "bash";
  const args = process.platform === "win32" ? ["-c", 'cd "$(cygpath -u "$BKUW_AUDIO_ROOT")" && bash scripts/audio/prepare.sh'] : ["scripts/audio/prepare.sh"];
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit", env: { ...process.env, BKUW_AUDIO_ROOT: root, MSYSTEM: "MINGW64", CHERE_INVOKING: "1" } });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
