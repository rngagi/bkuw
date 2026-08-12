import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, openUrlMock } = vi.hoisted(() => ({ invokeMock: vi.fn(), openUrlMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

import { backend } from "./tauri";

describe("Tauri adapter", () => {
  beforeEach(() => vi.clearAllMocks());

  it("accepts Tauri's null serialization for a Rust unit response", async () => {
    invokeMock.mockResolvedValue(null);
    await expect(backend.closeProject()).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenCalledWith("close_project", {});
  });

  it("opens only the documented ISO 15924 registry URL", async () => {
    openUrlMock.mockResolvedValue(undefined);
    await backend.openScriptCodeRegistry();
    expect(openUrlMock).toHaveBeenCalledWith("https://www.unicode.org/iso15924/iso15924-codes.html");
  });
});
