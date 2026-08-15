import { createHash } from "node:crypto";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { publishDraft, validateReleaseAssets } from "./publish-draft.mjs";

const roots: string[] = [];
const sha = "0123456789abcdef0123456789abcdef01234567";

function checksum(value: string) {
  return createHash("sha256").update(value).digest("hex");
}

async function releaseFixture() {
  const root = await mkdtemp(join(tmpdir(), "bkuw-publish-draft-"));
  roots.push(root);
  const assetsDirectory = join(root, "assets");
  await mkdir(assetsDirectory);
  await writeFile(join(assetsDirectory, "bkuw_0.4.3_aarch64.dmg"), "dmg");
  await writeFile(join(assetsDirectory, "bkuw_0.4.3_x64-setup.exe"), "exe");
  await writeFile(
    join(assetsDirectory, "SHA256SUMS.txt"),
    `${checksum("dmg")}  bkuw_0.4.3_aarch64.dmg\n${checksum("exe")}  bkuw_0.4.3_x64-setup.exe\n`,
  );
  const notesFile = join(root, "notes.md");
  await writeFile(notesFile, "## Downloads\n");
  return { repo: "rngagi/bkuw", tag: "v0.4.3", sha, assetsDirectory, notesFile };
}

afterEach(async () => {
  vi.restoreAllMocks();
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

describe("publish draft module", () => {
  it("creates a draft at the exact commit with explicit repository context", async () => {
    const input = await releaseFixture();
    const calls: string[][] = [];
    const runGh = vi.fn((args: string[]) => {
      calls.push(args);
      if (args[0] === "release" && args[1] === "view") {
        return { status: 1, stdout: "", stderr: "release not found" };
      }
      return { status: 0, stdout: "https://example.test/draft", stderr: "" };
    });

    await expect(publishDraft(input, { runGh })).resolves.toEqual({
      action: "created",
      url: "https://example.test/draft",
    });
    const create = calls.find((args) => args[0] === "release" && args[1] === "create");
    expect(create).toEqual(
      expect.arrayContaining([
        "--repo",
        "rngagi/bkuw",
        "--target",
        sha,
        "--draft",
        "--generate-notes",
      ]),
    );
  });

  it("updates an existing draft only when its tag targets the same commit", async () => {
    const input = await releaseFixture();
    const runGh = vi.fn((args: string[]) => {
      if (args[0] === "release" && args[1] === "view") {
        return {
          status: 0,
          stdout: JSON.stringify({
            isDraft: true,
            targetCommitish: sha,
            url: "https://example.test/draft",
          }),
          stderr: "",
        };
      }
      if (args[0] === "api") {
        return { status: 0, stdout: JSON.stringify({ type: "commit", sha }), stderr: "" };
      }
      return { status: 0, stdout: "", stderr: "" };
    });

    await expect(publishDraft(input, { runGh })).resolves.toMatchObject({ action: "updated" });
    expect(runGh).toHaveBeenCalledWith(
      expect.arrayContaining(["release", "upload", "v0.4.3", "--clobber", "--repo", "rngagi/bkuw"]),
    );
  });

  it("moves an unmaterialized draft tag to the newly tested commit before updating assets", async () => {
    const input = await releaseFixture();
    const previousSha = "abcdef0123456789abcdef0123456789abcdef01";
    const runGh = vi.fn((args: string[]) => {
      if (args[0] === "release" && args[1] === "view") {
        return {
          status: 0,
          stdout: JSON.stringify({
            isDraft: true,
            targetCommitish: previousSha,
            url: "https://example.test/draft",
          }),
          stderr: "",
        };
      }
      if (args[0] === "api") {
        return { status: 1, stdout: "", stderr: "HTTP 404: Not Found" };
      }
      return { status: 0, stdout: "", stderr: "" };
    });

    await expect(publishDraft(input, { runGh })).resolves.toMatchObject({ action: "updated" });
    expect(runGh).toHaveBeenCalledWith([
      "release",
      "edit",
      "v0.4.3",
      "--target",
      sha,
      "--repo",
      "rngagi/bkuw",
    ]);
    expect(runGh).toHaveBeenCalledWith(
      expect.arrayContaining(["release", "upload", "v0.4.3", "--clobber"]),
    );
  });

  it("refuses to replace a published release", async () => {
    const input = await releaseFixture();
    const runGh = vi.fn(() => ({
      status: 0,
      stdout: JSON.stringify({ isDraft: false, url: "https://example.test/release" }),
      stderr: "",
    }));

    await expect(publishDraft(input, { runGh })).rejects.toThrow("already published");
  });

  it("rejects incomplete or corrupted assets before calling GitHub", async () => {
    const input = await releaseFixture();
    await writeFile(join(input.assetsDirectory, "bkuw_0.4.3_x64-setup.exe"), "changed");
    const runGh = vi.fn();

    await expect(publishDraft(input, { runGh })).rejects.toThrow("SHA-256 mismatch");
    expect(runGh).not.toHaveBeenCalled();
  });

  it("requires exactly the supported release files", async () => {
    const input = await releaseFixture();
    await writeFile(join(input.assetsDirectory, "unexpected.zip"), "extra");
    await expect(validateReleaseAssets(input.assetsDirectory, "0.4.3")).rejects.toThrow(
      "Release assets must be exactly",
    );
  });
});
