import type { Image } from "../../ipc/bindings";

export type Attachment = { id: number; name: string; image: Image };

const mimeByExtension: Record<string, string> = { png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", gif: "image/gif", webp: "image/webp" };

export const imageExtensions = Object.keys(mimeByExtension);

export const isImageType = (type: string) => Object.values(mimeByExtension).includes(type);

export function imageMime(path: string): string | null {
  const dot = path.lastIndexOf(".");
  return dot < 0 ? null : (mimeByExtension[path.slice(dot + 1).toLowerCase()] ?? null);
}

const urls = new WeakMap<Image, string>();

export function imageUrl(image: Image): string {
  let url = urls.get(image);
  if (url === undefined) urls.set(image, (url = `data:${image.mime_type};base64,${image.data}`));
  return url;
}
