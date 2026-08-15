import { describe, expect, it } from "vitest";
import { arrayBufferToBase64, base64ToBytes, pngDataUrl, scaledImageDimensions } from "./imageCompression";

describe("scaledImageDimensions", () => {
  it("keeps ordinary images at their original resolution", () => {
    expect(scaledImageDimensions(1600, 1200)).toEqual({ width: 1600, height: 1200 });
  });

  it("lightly scales oversized images while preserving aspect ratio", () => {
    expect(scaledImageDimensions(6000, 4000)).toEqual({ width: 2560, height: 1707 });
    expect(scaledImageDimensions(3000, 6000)).toEqual({ width: 1280, height: 2560 });
  });

  it("round-trips binary PNG payloads through compact Base64 IPC", () => {
    const bytes = Uint8Array.from([0, 137, 80, 78, 71, 255]);
    expect(Array.from(base64ToBytes(arrayBufferToBase64(bytes.buffer)))).toEqual(Array.from(bytes));
  });
});

describe("pngDataUrl", () => {
  it("returns a CSP-compatible data URL for a PNG payload", () => {
    expect(pngDataUrl("iVBORw0KGgo=")).toBe("data:image/png;base64,iVBORw0KGgo=");
  });

  it("rejects invalid base64 and non-PNG payloads", () => {
    expect(() => pngDataUrl("not base64")).toThrow();
    expect(() => pngDataUrl("aGVsbG8=")).toThrow("image_invalid");
  });
});
