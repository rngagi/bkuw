const ACCEPTED_IMAGE_TYPES = new Set(["image/png", "image/jpeg", "image/webp"]);
const ACCEPTED_EXTENSIONS = /\.(png|jpe?g|webp)$/i;
export const SENSE_IMAGE_MAX_EDGE = 2560;

export interface PreparedSenseImage {
  originalFilename: string;
  pngBase64: string;
  width: number;
  height: number;
}

export function scaledImageDimensions(
  width: number,
  height: number,
  maxEdge = SENSE_IMAGE_MAX_EDGE,
) {
  if (width <= 0 || height <= 0) throw new Error("image_invalid");
  const scale = Math.min(1, maxEdge / Math.max(width, height));
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

export async function prepareSenseImage(file: File): Promise<PreparedSenseImage> {
  if (!(ACCEPTED_IMAGE_TYPES.has(file.type) || (!file.type && ACCEPTED_EXTENSIONS.test(file.name)))) {
    throw new Error("image_unsupported");
  }
  const decoded = await decodeImage(file);
  try {
    const dimensions = scaledImageDimensions(decoded.width, decoded.height);
    const canvas = document.createElement("canvas");
    canvas.width = dimensions.width;
    canvas.height = dimensions.height;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("image_processing");
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = "high";
    context.drawImage(decoded.source, 0, 0, dimensions.width, dimensions.height);
    const blob = await canvasToPng(canvas);
    return {
      originalFilename: file.name,
      pngBase64: arrayBufferToBase64(await blob.arrayBuffer()),
      width: dimensions.width,
      height: dimensions.height,
    };
  } finally {
    decoded.dispose();
  }
}

export function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  const chunkSize = 0x8000;
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + chunkSize));
  }
  return btoa(binary);
}

export function base64ToBytes(value: string): Uint8Array<ArrayBuffer> {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
  return bytes;
}

export function pngDataUrl(value: string): string {
  const bytes = base64ToBytes(value);
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (bytes.length < signature.length || signature.some((byte, index) => bytes[index] !== byte)) {
    throw new Error("image_invalid");
  }
  return `data:image/png;base64,${value}`;
}

async function canvasToPng(canvas: HTMLCanvasElement): Promise<Blob> {
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (!blob || blob.type !== "image/png") throw new Error("image_processing");
  return blob;
}

async function decodeImage(file: File): Promise<{
  source: CanvasImageSource;
  width: number;
  height: number;
  dispose(): void;
}> {
  if (typeof createImageBitmap === "function") {
    try {
      const bitmap = await createImageBitmap(file, { imageOrientation: "from-image" });
      return {
        source: bitmap,
        width: bitmap.width,
        height: bitmap.height,
        dispose: () => bitmap.close(),
      };
    } catch {
      // WebView implementations differ; the Image fallback uses the same local-only Canvas flow.
    }
  }
  const url = URL.createObjectURL(file);
  try {
    const image = await new Promise<HTMLImageElement>((resolve, reject) => {
      const element = new Image();
      element.onload = () => resolve(element);
      element.onerror = () => reject(new Error("image_invalid"));
      element.src = url;
    });
    return {
      source: image,
      width: image.naturalWidth,
      height: image.naturalHeight,
      dispose: () => URL.revokeObjectURL(url),
    };
  } catch (error) {
    URL.revokeObjectURL(url);
    throw error;
  }
}
