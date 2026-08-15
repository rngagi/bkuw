import { getCurrentWebview } from "@tauri-apps/api/webview";

const STORAGE_KEY = "bkuw.appZoom";
export const APP_ZOOM_LEVELS = [0.67, 0.8, 0.9, 1, 1.1, 1.25, 1.5] as const;

type ZoomShortcutEvent = Pick<
  KeyboardEvent,
  "altKey" | "code" | "ctrlKey" | "isComposing" | "key" | "metaKey" | "preventDefault"
>;

interface ZoomShortcutOptions {
  target: Pick<Window, "addEventListener" | "removeEventListener">;
  storage: Pick<Storage, "getItem" | "setItem">;
  setZoom(scaleFactor: number): Promise<void>;
  onError(error: unknown): void;
}

export function installTauriZoomShortcuts(onError: (error: unknown) => void): () => void {
  if (!("__TAURI_INTERNALS__" in window)) return () => undefined;
  return installZoomShortcuts({
    target: window,
    storage: window.localStorage,
    setZoom: (scaleFactor) => getCurrentWebview().setZoom(scaleFactor),
    onError,
  });
}

export function installZoomShortcuts(options: ZoomShortcutOptions): () => void {
  let current = storedZoom(options.storage);
  let active = true;
  let queue = Promise.resolve();

  function apply(scaleFactor: number) {
    current = scaleFactor;
    queue = queue
      .then(() => options.setZoom(scaleFactor))
      .then(() => {
        if (active) options.storage.setItem(STORAGE_KEY, String(scaleFactor));
      })
      .catch((error: unknown) => {
        if (active) options.onError(error);
      });
  }

  function onKeyDown(event: Event) {
    const keyboardEvent = event as KeyboardEvent;
    const direction = zoomShortcutDirection(keyboardEvent);
    if (direction === null) return;
    keyboardEvent.preventDefault();
    apply(direction === 0 ? 1 : adjacentZoom(current, direction));
  }

  apply(current);
  options.target.addEventListener("keydown", onKeyDown);
  return () => {
    active = false;
    options.target.removeEventListener("keydown", onKeyDown);
  };
}

export function zoomShortcutDirection(event: ZoomShortcutEvent): -1 | 0 | 1 | null {
  if (event.isComposing || event.altKey || !(event.ctrlKey || event.metaKey)) return null;
  if (event.key === "0" || event.code === "Numpad0") return 0;
  if (["=", "+", "Add"].includes(event.key) || ["Equal", "NumpadAdd"].includes(event.code)) return 1;
  if (["-", "Subtract"].includes(event.key) || ["Minus", "NumpadSubtract"].includes(event.code)) return -1;
  return null;
}

export function adjacentZoom(current: number, direction: -1 | 1): number {
  const exact = APP_ZOOM_LEVELS.indexOf(current as typeof APP_ZOOM_LEVELS[number]);
  const index = exact >= 0
    ? exact
    : APP_ZOOM_LEVELS.reduce((best, value, candidate) => (
      Math.abs(value - current) < Math.abs(APP_ZOOM_LEVELS[best] - current) ? candidate : best
    ), 0);
  return APP_ZOOM_LEVELS[Math.max(0, Math.min(APP_ZOOM_LEVELS.length - 1, index + direction))];
}

function storedZoom(storage: Pick<Storage, "getItem">): number {
  const value = Number(storage.getItem(STORAGE_KEY));
  return APP_ZOOM_LEVELS.includes(value as typeof APP_ZOOM_LEVELS[number]) ? value : 1;
}
