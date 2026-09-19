import { beforeEach, describe, expect, it, vi } from "vitest";
import mainWindowCapability from "../../src-tauri/capabilities/default.json";

const { invokeMock, openUrlMock } = vi.hoisted(() => ({ invokeMock: vi.fn(), openUrlMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

import { backend, exportHelpUrls } from "./tauri";

describe("Tauri adapter", () => {
  beforeEach(() => vi.clearAllMocks());

  it("accepts Tauri's null serialization for a Rust unit response", async () => {
    invokeMock.mockResolvedValue(null);
    await expect(backend.closeProject()).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenCalledWith("close_project", {});
  });

  it("allows the main window to force-close after flushing and closing the project", () => {
    expect(mainWindowCapability.windows).toEqual(["main"]);
    expect(mainWindowCapability.permissions).toContain("core:window:allow-destroy");
  });

  it("opens only the documented ISO 15924 registry URL", async () => {
    openUrlMock.mockResolvedValue(undefined);
    await backend.openScriptCodeRegistry();
    expect(openUrlMock).toHaveBeenCalledWith("https://www.unicode.org/iso15924/iso15924-codes.html");
  });
  it("validates environment results and allows every documented help link", async () => {
    invokeMock.mockResolvedValue({ state: "missingPackages", path: "C:\\TeX\\xelatex.exe", version: "XeTeX", missingFiles: ["fancybox.sty"], diagnosticPath: null });
    await expect(backend.checkLatexEnvironment()).resolves.toMatchObject({ state: "missingPackages", missingFiles: ["fancybox.sty"] });
    invokeMock.mockResolvedValue({ state: "unknown" });
    await expect(backend.checkLatexEnvironment()).rejects.toThrow();
    const opener = mainWindowCapability.permissions.find((permission) => typeof permission === "object" && permission.identifier === "opener:allow-open-url") as { allow: { url: string }[] };
    for (const topic of Object.keys(exportHelpUrls) as (keyof typeof exportHelpUrls)[]) {
      await backend.openExportHelp(topic);
      expect(openUrlMock).toHaveBeenLastCalledWith(exportHelpUrls[topic]);
      expect(opener.allow).toContainEqual({ url: exportHelpUrls[topic] });
    }
  });

});
