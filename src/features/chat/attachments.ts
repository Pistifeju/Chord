// Rules for images attached to a message: which files and clipboard types count as images, and how an image shows.
import type { Image } from "../../ipc/bindings";

/** An image waiting in the composer. */
export type Attachment = { id: number; name: string; image: Image };

const mimeByExtension: Record<string, string> = { png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", gif: "image/gif", webp: "image/webp" };

export const imageExtensions = Object.keys(mimeByExtension);

export const isImageType = (type: string) => Object.values(mimeByExtension).includes(type);

/** The MIME type of an image file by its extension; null for any other file. */
export function imageMime(path: string): string | null {
  const dot = path.lastIndexOf(".");
  return dot < 0 ? null : (mimeByExtension[path.slice(dot + 1).toLowerCase()] ?? null);
}

export const fileName = (path: string) => path.slice(path.lastIndexOf("/") + 1);

export const imageUrl = (image: Image) => `data:${image.mime_type};base64,${image.data}`;
