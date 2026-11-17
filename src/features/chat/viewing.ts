// The image viewer state: which pane shows which image of a message at full size.
import type { Image } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";

type Viewing = { images: Image[]; index: number };

export const lightboxes = externalStore<Record<string, Viewing>>({});

export const openLightbox = (pane: string, images: Image[], index: number) => lightboxes.set({ ...lightboxes.get(), [pane]: { images, index } });

export function stepLightbox(pane: string, by: -1 | 1) {
  const open = lightboxes.get()[pane];
  if (open) openLightbox(pane, open.images, Math.min(open.images.length - 1, Math.max(0, open.index + by)));
}

export function closeLightbox(pane: string) {
  const { [pane]: _closed, ...rest } = lightboxes.get();
  lightboxes.set(rest);
}
