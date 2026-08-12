import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { backend } from "./tauri";

describe("Tauri adapter", () => {
  beforeEach(() => vi.clearAllMocks());

  it("accepts Tauri's null serialization for a Rust unit response", async () => {
    invokeMock.mockResolvedValue(null);
    await expect(backend.closeProject()).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenCalledWith("close_project", {});
  });
});
