import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

const { backendMock } = vi.hoisted(() => ({
  backendMock: { listFontPacks: vi.fn(), installFontPacks: vi.fn() },
}));
vi.mock("../../lib/tauri", () => ({
  backend: backendMock,
  CommandError: class CommandError extends Error {
    constructor(public code: string, message: string, public details?: string) { super(message); }
  },
}));

import { FontSetup } from "./FontSetup";

const missing = [
  { id: "tex-gyre-termes", version: "2.004", state: "missing", mandatory: true, installedBytes: 0 },
  { id: "chiron-sung-hk", version: "2.609", state: "installed", mandatory: false, installedBytes: 10 },
];

describe("FontSetup", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    vi.clearAllMocks();
    backendMock.listFontPacks.mockResolvedValue(missing);
  });

  it("installs every pending pack and reports byte progress", async () => {
    const onContinue = vi.fn();
    backendMock.installFontPacks.mockImplementation(async (_ids, onProgress) => {
      onProgress({ packId: "tex-gyre-termes", phase: "downloading", packIndex: 0, packCount: 1, downloadedBytes: 50, totalBytes: 100 });
      onProgress({ packId: "tex-gyre-termes", phase: "installed", packIndex: 0, packCount: 1, downloadedBytes: 100, totalBytes: 100 });
      backendMock.listFontPacks.mockResolvedValue(missing.map((pack) => ({ ...pack, state: "installed" })));
      return [];
    });
    render(<FontSetup onContinue={onContinue} />);
    fireEvent.click(await screen.findByRole("button", { name: "Download all fonts" }));
    await waitFor(() => expect(backendMock.installFontPacks).toHaveBeenCalledWith(["tex-gyre-termes"], expect.any(Function)));
    await waitFor(() => expect(onContinue).toHaveBeenCalled());
  });

  it("only reveals offline entry after a real failure", async () => {
    backendMock.installFontPacks.mockRejectedValue(new Error("offline"));
    render(<FontSetup onContinue={vi.fn()} />);
    expect(screen.queryByRole("button", { name: "Use bkuw offline" })).not.toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Download all fonts" }));
    expect(await screen.findByRole("button", { name: "Use bkuw offline" })).toBeInTheDocument();
  });
});
