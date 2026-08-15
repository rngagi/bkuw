import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = resolve(import.meta.dirname, "../..");

describe("release workflow invariants", () => {
  it("keeps normal main CI free of installer artifacts", async () => {
    const ci = await readFile(resolve(root, ".github/workflows/ci.yml"), "utf8");
    expect(ci).not.toContain("upload-artifact");
    expect(ci).toContain("--no-bundle");
    expect(ci).not.toContain("x86_64-apple-darwin");
  });

  it("prepares releases after successful CI instead of accepting pushed tags", async () => {
    const release = await readFile(resolve(root, ".github/workflows/release.yml"), "utf8");
    expect(release).toContain("workflow_run:");
    expect(release).toContain("workflow_dispatch:");
    expect(release).not.toMatch(/push:\s*\n\s+tags:/);
    expect(release).toContain("github.event.workflow_run.conclusion == 'success'");
    expect(release).toContain("github.event.workflow_run.head_repository.full_name == github.repository");
    expect(release).toContain("fetch-depth: 0");
    expect(release).toContain("git log -2 --format=%H -- package.json");
  });

  it("builds only supported installers and creates a draft at the exact SHA", async () => {
    const release = await readFile(resolve(root, ".github/workflows/release.yml"), "utf8");
    expect(release).toContain("aarch64-apple-darwin");
    expect(release).toContain("x86_64-pc-windows-msvc");
    expect(release).not.toContain("x86_64-apple-darwin");
    expect(release).toContain("publish-draft.mjs");
    expect(release).toContain('--sha "${{ needs.plan-release.outputs.target_sha }}"');
  });

  it("supports recovery from an earlier installer run", async () => {
    const release = await readFile(resolve(root, ".github/workflows/release.yml"), "utf8");
    expect(release).toContain("source_run_id:");
    expect(release).toContain("artifact_run_id");
    expect(release).toContain("run-id: ${{ needs.plan-release.outputs.artifact_run_id }}");
  });
});
