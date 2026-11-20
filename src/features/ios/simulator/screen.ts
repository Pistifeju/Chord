// Rules for the Simulator view's screen: where it fits in the view, its corner, a pointer as a point on it, and the fingers a click, a drag or a trackpad gesture puts down.
/** A point on the screen, 0…1 from its top left. */
export type Point = { x: number; y: number };

export type Rect = { x: number; y: number; width: number; height: number };

/** A touch that starts this close to the bottom is a swipe from the bottom edge: Home on Face ID models. */
const BOTTOM_EDGE = 0.97;
/** Half the distance between a trackpad gesture's two fingers before it scales. */
const GESTURE_REACH = 0.1;
/** The screen's corner as a share of its short side: close to an iPhone's display, and to an iPad's, which rounds far less. */
const PHONE_CORNER = 0.12;
const TABLET_CORNER = 0.022;
/** Long side over short side above which a screen is an iPhone's (iPhones run 1.78 to 2.17, iPads 1.33 to 1.56). */
const PHONE_SHAPE = 1.7;

/** The screen, `width` × `height` pixels, as large as fits in `box` and centered, keeping its shape. */
export function fit(box: Rect, { width, height }: { width: number; height: number }): Rect {
  if (box.width <= 0 || box.height <= 0 || width <= 0 || height <= 0) return { x: box.x, y: box.y, width: 0, height: 0 };
  const scale = Math.min(box.width / width, box.height / height);
  const fitted = { width: Math.round(width * scale), height: Math.round(height * scale) };
  return { x: Math.round(box.x + (box.width - fitted.width) / 2), y: Math.round(box.y + (box.height - fitted.height) / 2), ...fitted };
}

export function cornerRadius({ width, height }: Rect): number {
  const short = Math.min(width, height);
  if (short <= 0) return 0;
  return Math.round(short * (Math.max(width, height) / short > PHONE_SHAPE ? PHONE_CORNER : TABLET_CORNER));
}

const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

/** A pointer at `x`, `y` as a point on the screen, 0…1 from its top left; null outside it. */
export function toScreen(x: number, y: number, rect: Rect): Point | null {
  if (rect.width <= 0 || rect.height <= 0) return null;
  const point = { x: (x - rect.x) / rect.width, y: (y - rect.y) / rect.height };
  return point.x >= 0 && point.x <= 1 && point.y >= 0 && point.y <= 1 ? point : null;
}

/** Like `toScreen`, pinned to the edge: a drag that leaves the screen keeps touching its edge. */
export function clampToScreen(x: number, y: number, rect: Rect): Point {
  if (rect.width <= 0 || rect.height <= 0) return { x: 0.5, y: 0.5 };
  return { x: clamp01((x - rect.x) / rect.width), y: clamp01((y - rect.y) / rect.height) };
}

export const fromBottom = (point: Point) => point.y >= BOTTOM_EDGE;

/** One finger; ⌥: a second one mirrored around the screen's center (pinch, rotate); ⌥⇧: a second one that keeps its offset from the first (two-finger drag). */
export type Fingers = { kind: "one" } | { kind: "pinch" } | { kind: "pan"; offset: Point };

const mirror = (point: Point): Point => ({ x: 1 - point.x, y: 1 - point.y });

/** The fingers a press at `first` puts down with these keys held. The pan's second finger starts where the mirror would put it. */
export function fingersFor(keys: { alt: boolean; shift: boolean }, first: Point): Fingers {
  if (!keys.alt) return { kind: "one" };
  if (!keys.shift) return { kind: "pinch" };
  const second = mirror(first);
  return { kind: "pan", offset: { x: second.x - first.x, y: second.y - first.y } };
}

export function secondFinger(fingers: Fingers, first: Point): Point | null {
  if (fingers.kind === "one") return null;
  if (fingers.kind === "pinch") return mirror(first);
  return { x: clamp01(first.x + fingers.offset.x), y: clamp01(first.y + fingers.offset.y) };
}

/** A trackpad pinch or rotate as two fingers around `center`: `scale` times their start distance apart, turned `rotation` degrees clockwise. */
export function gestureFingers(center: Point, scale: number, rotation: number): [Point, Point] {
  const angle = (rotation * Math.PI) / 180;
  const reach = GESTURE_REACH * scale;
  const dx = Math.cos(angle) * reach;
  const dy = Math.sin(angle) * reach;
  return [
    { x: clamp01(center.x - dx), y: clamp01(center.y - dy) },
    { x: clamp01(center.x + dx), y: clamp01(center.y + dy) },
  ];
}
