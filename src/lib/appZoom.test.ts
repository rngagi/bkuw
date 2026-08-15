import { afterEach, describe, expect, it, vi } from "vitest";
import { adjacentZoom, installZoomShortcuts, zoomShortcutDirection } from "./appZoom";

const cleanups: Array<() => void> = [];

afterEach(() => {
  cleanups.splice(0).forEach((cleanup) => cleanup());
  window.localStorage.clear();
});

describe("app zoom shortcuts", () => {
  it("recognizes Windows, macOS, shifted equals, reset, and numpad keys", () => {
    const event = (value: Partial<KeyboardEvent>) => ({
      altKey: false, code: "", ctrlKey: false, isComposing: false,
      key: "", metaKey: false, preventDefault: vi.fn(), ...value,
    });
    expect(zoomShortcutDirection(event({ key: "-", ctrlKey: true }))).toBe(-1);
    expect(zoomShortcutDirection(event({ key: "=", metaKey: true }))).toBe(1);
    expect(zoomShortcutDirection(event({ key: "+", ctrlKey: true }))).toBe(1);
    expect(zoomShortcutDirection(event({ key: "0", metaKey: true }))).toBe(0);
    expect(zoomShortcutDirection(event({ code: "NumpadSubtract", ctrlKey: true }))).toBe(-1);
    expect(zoomShortcutDirection(event({ key: "=", ctrlKey: true, isComposing: true }))).toBeNull();
    expect(zoomShortcutDirection(event({ key: "=", ctrlKey: true, altKey: true }))).toBeNull();
    expect(zoomShortcutDirection(event({ key: "s", ctrlKey: true }))).toBeNull();
  });

  it("moves through bounded readable levels", () => {
    expect(adjacentZoom(1, -1)).toBe(0.9);
    expect(adjacentZoom(1, 1)).toBe(1.1);
    expect(adjacentZoom(0.67, -1)).toBe(0.67);
    expect(adjacentZoom(1.5, 1)).toBe(1.5);
  });

  it("applies and persists zoom while leaving other shortcuts alone", async () => {
    window.localStorage.setItem("bkuw.appZoom", "0.8");
    const setZoom = vi.fn().mockResolvedValue(undefined);
    const onError = vi.fn();
    cleanups.push(installZoomShortcuts({ target: window, storage: window.localStorage, setZoom, onError }));
    await vi.waitFor(() => expect(setZoom).toHaveBeenLastCalledWith(0.8));

    const zoomIn = new KeyboardEvent("keydown", { key: "=", ctrlKey: true, cancelable: true });
    window.dispatchEvent(zoomIn);
    expect(zoomIn.defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(setZoom).toHaveBeenLastCalledWith(0.9));
    expect(window.localStorage.getItem("bkuw.appZoom")).toBe("0.9");

    const reset = new KeyboardEvent("keydown", { key: "0", metaKey: true, cancelable: true });
    window.dispatchEvent(reset);
    await vi.waitFor(() => expect(setZoom).toHaveBeenLastCalledWith(1));

    const save = new KeyboardEvent("keydown", { key: "s", ctrlKey: true, cancelable: true });
    window.dispatchEvent(save);
    expect(save.defaultPrevented).toBe(false);
    expect(onError).not.toHaveBeenCalled();
  });

  it("reports WebView zoom failures", async () => {
    const failure = new Error("zoom denied");
    const onError = vi.fn();
    cleanups.push(installZoomShortcuts({
      target: window,
      storage: window.localStorage,
      setZoom: vi.fn().mockRejectedValue(failure),
      onError,
    }));
    await vi.waitFor(() => expect(onError).toHaveBeenCalledWith(failure));
  });
});
