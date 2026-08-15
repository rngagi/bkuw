import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

function defaultRunGh(args) {
  const result = spawnSync("gh", args, { encoding: "utf8" });
  if (result.error) {
    throw result.error;
  }
  return {
    status: result.status ?? 1,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
  };
}

function assertSuccessful(result, description) {
  if (result.status !== 0) {
    throw new Error(`${description} failed: ${result.stderr.trim() || result.stdout.trim()}`);
  }
  return result.stdout.trim();
}

async function sha256(filename) {
  return createHash("sha256").update(await readFile(filename)).digest("hex");
}

export async function validateReleaseAssets(assetsDirectory, version) {
  const expectedNames = [
    `bkuw_${version}_aarch64.dmg`,
    `bkuw_${version}_x64-setup.exe`,
    "SHA256SUMS.txt",
  ];
  const actualNames = (await readdir(assetsDirectory)).sort();
  if (JSON.stringify(actualNames) !== JSON.stringify([...expectedNames].sort())) {
    throw new Error(
      `Release assets must be exactly ${expectedNames.join(", ")}; found ${actualNames.join(", ") || "none"}`,
    );
  }

  const checksumContents = await readFile(resolve(assetsDirectory, "SHA256SUMS.txt"), "utf8");
  const checksumEntries = new Map(
    checksumContents
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => {
        const match = line.match(/^([a-f0-9]{64})\s+\*?(.+)$/);
        if (!match) {
          throw new Error(`Invalid SHA256SUMS.txt line: ${line}`);
        }
        return [match[2], match[1]];
      }),
  );
  for (const name of expectedNames.slice(0, 2)) {
    const actual = await sha256(resolve(assetsDirectory, name));
    if (checksumEntries.get(name) !== actual) {
      throw new Error(`SHA-256 mismatch for ${name}`);
    }
  }
  if (checksumEntries.size !== 2) {
    throw new Error("SHA256SUMS.txt must contain exactly the DMG and EXE checksums");
  }

  return expectedNames.map((name) => resolve(assetsDirectory, name));
}

async function resolveTagCommit(repo, tag, runGh) {
  let object = JSON.parse(
    assertSuccessful(
      runGh(["api", `repos/${repo}/git/ref/tags/${tag}`, "--jq", ".object"]),
      `Reading ${tag}`,
    ),
  );
  for (let depth = 0; object.type === "tag" && depth < 4; depth += 1) {
    object = JSON.parse(
      assertSuccessful(
        runGh(["api", `repos/${repo}/git/tags/${object.sha}`, "--jq", ".object"]),
        `Resolving ${tag}`,
      ),
    );
  }
  if (object.type !== "commit" || typeof object.sha !== "string") {
    throw new Error(`Tag ${tag} does not resolve to a commit`);
  }
  return object.sha;
}

export async function publishDraft(
  { repo, tag, sha, assetsDirectory, notesFile },
  { runGh = defaultRunGh } = {},
) {
  if (!/^[\w.-]+\/[\w.-]+$/.test(repo)) {
    throw new Error(`Invalid repository: ${repo}`);
  }
  if (!/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(tag)) {
    throw new Error(`Invalid release tag: ${tag}`);
  }
  if (!/^[a-f0-9]{40}$/.test(sha)) {
    throw new Error(`Invalid release commit SHA: ${sha}`);
  }
  const version = tag.slice(1);
  const assets = await validateReleaseAssets(assetsDirectory, version);
  await readFile(notesFile, "utf8");

  const releaseView = runGh([
    "release",
    "view",
    tag,
    "--repo",
    repo,
    "--json",
    "isDraft,url",
  ]);
  if (releaseView.status === 0) {
    const release = JSON.parse(releaseView.stdout);
    if (!release.isDraft) {
      throw new Error(`Release ${tag} is already published and cannot be replaced`);
    }
    const tagCommit = await resolveTagCommit(repo, tag, runGh);
    if (tagCommit !== sha) {
      throw new Error(`Release ${tag} points to ${tagCommit}, expected ${sha}`);
    }
    assertSuccessful(
      runGh(["release", "upload", tag, ...assets, "--clobber", "--repo", repo]),
      `Updating draft ${tag}`,
    );
    return { action: "updated", url: release.url };
  }
  if (!/not found/i.test(`${releaseView.stderr}\n${releaseView.stdout}`)) {
    throw new Error(`Checking release ${tag} failed: ${releaseView.stderr.trim()}`);
  }

  const createdUrl = assertSuccessful(
    runGh([
      "release",
      "create",
      tag,
      ...assets,
      "--repo",
      repo,
      "--target",
      sha,
      "--draft",
      "--generate-notes",
      "--fail-on-no-commits",
      "--title",
      `bkuw ${tag}`,
      "--notes-file",
      notesFile,
    ]),
    `Creating draft ${tag}`,
  );
  return { action: "created", url: createdUrl };
}

function parseArguments(args) {
  const values = new Map();
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    const value = args[index + 1];
    if (!key?.startsWith("--") || !value) {
      throw new Error(
        "Usage: publish-draft --repo OWNER/REPO --tag vX.Y.Z --sha COMMIT --assets-dir DIR --notes-file FILE",
      );
    }
    values.set(key.slice(2), value);
  }
  return {
    repo: values.get("repo"),
    tag: values.get("tag"),
    sha: values.get("sha"),
    assetsDirectory: resolve(values.get("assets-dir") ?? ""),
    notesFile: resolve(values.get("notes-file") ?? ""),
  };
}

async function runCli() {
  if (!process.env.GH_TOKEN) {
    throw new Error("GH_TOKEN is required to publish a draft release");
  }
  const result = await publishDraft(parseArguments(process.argv.slice(2)));
  console.log(`${result.action} draft release: ${result.url}`);
}

const isCli = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isCli) {
  runCli().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
