import { execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const STABLE_VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const VERSION_FILES = [
  "package.json",
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
  "src-tauri/tauri.conf.json",
];

function parseVersion(version) {
  const match = STABLE_VERSION.exec(version);
  if (!match) {
    throw new Error(`Expected a stable semantic version such as 0.4.3, received: ${version}`);
  }
  return match.slice(1).map(Number);
}

function compareVersions(left, right) {
  const leftParts = parseVersion(left);
  const rightParts = parseVersion(right);
  for (let index = 0; index < leftParts.length; index += 1) {
    if (leftParts[index] !== rightParts[index]) {
      return leftParts[index] - rightParts[index];
    }
  }
  return 0;
}

function replaceCargoPackageVersion(contents, version) {
  const packageSection = contents.match(/(^\[package\]\n)([\s\S]*?)(?=^\[|(?![\s\S]))/m);
  if (!packageSection) {
    throw new Error("Cargo.toml does not contain a [package] section");
  }
  const versionMatches = packageSection[2].match(/^version = "[^"]+"$/gm) ?? [];
  if (versionMatches.length !== 1) {
    throw new Error("Cargo.toml [package] must contain exactly one version");
  }
  const replacement = packageSection[2].replace(/^version = "[^"]+"$/m, `version = "${version}"`);
  return contents.replace(packageSection[0], `${packageSection[1]}${replacement}`);
}

function findCargoLockPackage(contents) {
  const blocks = [
    ...contents.matchAll(/^\[\[package\]\]\n[\s\S]*?(?=^\[\[package\]\]|(?![\s\S]))/gm),
  ];
  const matches = blocks.filter((match) => /^name = "bkuw"$/m.test(match[0]));
  if (matches.length !== 1) {
    throw new Error("Cargo.lock must contain exactly one bkuw package");
  }
  const versionMatches = matches[0][0].match(/^version = "[^"]+"$/gm) ?? [];
  if (versionMatches.length !== 1) {
    throw new Error("The bkuw Cargo.lock package must contain exactly one version");
  }
  return matches[0][0];
}

function replaceCargoLockVersion(contents, version) {
  const packageBlock = findCargoLockPackage(contents);
  return contents.replace(
    packageBlock,
    packageBlock.replace(/^version = "[^"]+"$/m, `version = "${version}"`),
  );
}

function jsonVersion(contents, filename) {
  const parsed = JSON.parse(contents);
  if (typeof parsed.version !== "string") {
    throw new Error(`${filename} must contain a string version`);
  }
  parseVersion(parsed.version);
  return parsed.version;
}

function replaceJsonVersion(contents, filename, version) {
  jsonVersion(contents, filename);
  const matches = contents.match(/^(\s*"version"\s*:\s*")[^"]+("\s*,?\s*)$/gm) ?? [];
  if (matches.length !== 1) {
    throw new Error(`${filename} must contain exactly one top-level version field`);
  }
  return contents.replace(
    /^(\s*"version"\s*:\s*")[^"]+("\s*,?\s*)$/m,
    `$1${version}$2`,
  );
}

function cargoTomlVersion(contents) {
  const packageSection = contents.match(/(^\[package\]\n)([\s\S]*?)(?=^\[|(?![\s\S]))/m);
  const match = packageSection?.[2].match(/^version = "([^"]+)"$/m);
  if (!match) {
    throw new Error("Cargo.toml [package] must contain exactly one version");
  }
  parseVersion(match[1]);
  return match[1];
}

function cargoLockVersion(contents) {
  const packageBlock = findCargoLockPackage(contents);
  const match = packageBlock.match(/^version = "([^"]+)"$/m);
  parseVersion(match[1]);
  return match[1];
}

async function loadVersionFiles(root) {
  const entries = await Promise.all(
    VERSION_FILES.map(async (filename) => [filename, await readFile(join(root, filename), "utf8")]),
  );
  return Object.fromEntries(entries);
}

export async function readReleaseVersion(root) {
  const files = await loadVersionFiles(root);
  const versions = {
    "package.json": jsonVersion(files["package.json"], "package.json"),
    "src-tauri/Cargo.toml": cargoTomlVersion(files["src-tauri/Cargo.toml"]),
    "src-tauri/Cargo.lock": cargoLockVersion(files["src-tauri/Cargo.lock"]),
    "src-tauri/tauri.conf.json": jsonVersion(
      files["src-tauri/tauri.conf.json"],
      "src-tauri/tauri.conf.json",
    ),
  };
  const uniqueVersions = new Set(Object.values(versions));
  if (uniqueVersions.size !== 1) {
    throw new Error(
      `Release versions are inconsistent:\n${Object.entries(versions)
        .map(([filename, version]) => `- ${filename}: ${version}`)
        .join("\n")}`,
    );
  }
  return { version: Object.values(versions)[0], files };
}

async function writeVersionFiles(root, outputs) {
  const temporaryFiles = [];
  try {
    for (const [filename, contents] of Object.entries(outputs)) {
      const target = join(root, filename);
      const temporary = join(dirname(target), `.bkuw-release-${randomUUID()}.tmp`);
      await writeFile(temporary, contents, "utf8");
      temporaryFiles.push([temporary, target]);
    }
    for (const [temporary, target] of temporaryFiles) {
      await rename(temporary, target);
    }
  } finally {
    await Promise.all(temporaryFiles.map(([temporary]) => rm(temporary, { force: true })));
  }
}

export async function prepareReleaseVersion(root, nextVersion) {
  parseVersion(nextVersion);
  const { version: currentVersion, files } = await readReleaseVersion(root);
  if (compareVersions(nextVersion, currentVersion) <= 0) {
    throw new Error(`The release version must increase from ${currentVersion}; received ${nextVersion}`);
  }

  await writeVersionFiles(root, {
    "package.json": replaceJsonVersion(files["package.json"], "package.json", nextVersion),
    "src-tauri/Cargo.toml": replaceCargoPackageVersion(
      files["src-tauri/Cargo.toml"],
      nextVersion,
    ),
    "src-tauri/Cargo.lock": replaceCargoLockVersion(files["src-tauri/Cargo.lock"], nextVersion),
    "src-tauri/tauri.conf.json": replaceJsonVersion(
      files["src-tauri/tauri.conf.json"],
      "src-tauri/tauri.conf.json",
      nextVersion,
    ),
  });

  return { currentVersion, nextVersion, files: VERSION_FILES };
}

function git(root, args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}

function assertReleaseWorktree(root) {
  const branch = git(root, ["branch", "--show-current"]);
  if (branch !== "main") {
    throw new Error(`Release preparation must run on main; current branch: ${branch || "detached HEAD"}`);
  }
  const status = git(root, ["status", "--porcelain"]);
  if (status) {
    throw new Error("Release preparation requires a clean worktree; commit or stash changes first");
  }
}

async function runCli() {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
  const argumentsWithoutSeparator = process.argv.slice(2).filter((argument) => argument !== "--");
  const [command, expectedVersion, ...extra] = argumentsWithoutSeparator;
  if (extra.length > 0 || !["check", "prepare"].includes(command)) {
    throw new Error("Usage: pnpm release:prepare -- <version> | pnpm release:check -- [version]");
  }

  if (command === "check") {
    const { version } = await readReleaseVersion(root);
    if (expectedVersion && version !== expectedVersion) {
      throw new Error(`Expected release version ${expectedVersion}, found ${version}`);
    }
    console.log(`bkuw release version ${version} is consistent.`);
    return;
  }

  if (!expectedVersion) {
    throw new Error("release:prepare requires a version such as 0.4.3");
  }
  assertReleaseWorktree(root);
  const result = await prepareReleaseVersion(root, expectedVersion);
  console.log(`Prepared bkuw ${result.nextVersion} from ${result.currentVersion}.`);
  for (const filename of result.files) {
    console.log(`- ${filename}`);
  }
  console.log("Review the changes, commit them, and push main. Successful CI will prepare a draft release.");
}

const isCli = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isCli) {
  runCli().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
