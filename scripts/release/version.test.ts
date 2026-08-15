import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { prepareReleaseVersion, readReleaseVersion } from "./version.mjs";

const temporaryRoots: string[] = [];

async function fixture(version = "0.4.2") {
  const root = await mkdtemp(join(tmpdir(), "bkuw-release-version-"));
  temporaryRoots.push(root);
  await mkdir(join(root, "src-tauri"));
  await writeFile(
    join(root, "package.json"),
    `${JSON.stringify({ name: "bkuw", version, private: true }, null, 2)}\n`,
  );
  await writeFile(
    join(root, "src-tauri/Cargo.toml"),
    `[package]\nname = "bkuw"\nversion = "${version}"\nedition = "2024"\n\n[dependencies]\nserde = "1"\n`,
  );
  await writeFile(
    join(root, "src-tauri/Cargo.lock"),
    `version = 4\n\n[[package]]\nname = "bkuw"\nversion = "${version}"\ndependencies = [\n "serde",\n]\n\n[[package]]\nname = "serde"\nversion = "1.0.0"\n`,
  );
  await writeFile(
    join(root, "src-tauri/tauri.conf.json"),
    `${JSON.stringify({ productName: "bkuw", version }, null, 2)}\n`,
  );
  return root;
}

afterEach(async () => {
  const { rm } = await import("node:fs/promises");
  await Promise.all(temporaryRoots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

describe("release version module", () => {
  it("updates every canonical version file together", async () => {
    const root = await fixture();
    const tauriPath = join(root, "src-tauri/tauri.conf.json");
    await writeFile(
      tauriPath,
      '{\n  "productName": "bkuw",\n  "version": "0.4.2",\n  "capabilities": ["default"]\n}\n',
    );

    const result = await prepareReleaseVersion(root, "0.4.3");

    expect(result).toMatchObject({ currentVersion: "0.4.2", nextVersion: "0.4.3" });
    await expect(readReleaseVersion(root)).resolves.toMatchObject({ version: "0.4.3" });
    expect(await readFile(join(root, "src-tauri/Cargo.toml"), "utf8")).toContain(
      'version = "0.4.3"',
    );
    expect(await readFile(join(root, "src-tauri/Cargo.lock"), "utf8")).toContain(
      'name = "bkuw"\nversion = "0.4.3"',
    );
    expect(await readFile(tauriPath, "utf8")).toContain('"capabilities": ["default"]');
  });

  it("rejects inconsistent source versions before writing", async () => {
    const root = await fixture();
    const tauriPath = join(root, "src-tauri/tauri.conf.json");
    await writeFile(tauriPath, '{"productName":"bkuw","version":"0.4.1"}\n');
    const packageBefore = await readFile(join(root, "package.json"), "utf8");

    await expect(prepareReleaseVersion(root, "0.4.3")).rejects.toThrow(
      "Release versions are inconsistent",
    );
    await expect(readFile(join(root, "package.json"), "utf8")).resolves.toBe(packageBefore);
  });

  it("reads bkuw when it is the final Cargo.lock package", async () => {
    const root = await fixture();
    await writeFile(
      join(root, "src-tauri/Cargo.lock"),
      'version = 4\n\n[[package]]\nname = "serde"\nversion = "1.0.0"\n\n[[package]]\nname = "bkuw"\nversion = "0.4.2"\n',
    );

    await expect(readReleaseVersion(root)).resolves.toMatchObject({ version: "0.4.2" });
  });

  it.each(["0.4.2", "0.4.1", "v0.4.3", "0.4", "0.4.3-beta.1"])(
    "rejects a non-increasing or unsupported target %s",
    async (target) => {
      const root = await fixture();
      await expect(prepareReleaseVersion(root, target)).rejects.toThrow();
    },
  );
});
