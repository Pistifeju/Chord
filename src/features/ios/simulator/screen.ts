export type Point = { x: number; y: number };

export type Rect = { x: number; y: number; width: number; height: number };

const BOTTOM_EDGE = 0.97;
const GESTURE_REACH = 0.1;
const PHONE_CORNER = 0.12;
const TABLET_CORNER = 0.022;
const PHONE_SHAPE = 1.7;

export function fit(box: Rect, { width, height }: { width: number; height: number }, maxScale = Infinity): Rect {
  if (box.width <= 0 || box.height <= 0 || width <= 0 || height <= 0) return { x: box.x, y: box.y, width: 0, height: 0 };
  const scale = Math.min(maxScale, box.width / width, box.height / height);
  const fitted = { width: Math.round(width * scale), height: Math.round(height * scale) };
  return { x: Math.round(box.x + (box.width - fitted.width) / 2), y: Math.round(box.y + (box.height - fitted.height) / 2), ...fitted };
}

export function cornerRadius({ width, height }: Rect): number {
  const short = Math.min(width, height);
  if (short <= 0) return 0;
  return Math.round(short * (Math.max(width, height) / short > PHONE_SHAPE ? PHONE_CORNER : TABLET_CORNER));
}

const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

export function toScreen(x: number, y: number, rect: Rect): Point | null {
  if (rect.width <= 0 || rect.height <= 0) return null;
  const point = { x: (x - rect.x) / rect.width, y: (y - rect.y) / rect.height };
  return point.x >= 0 && point.x <= 1 && point.y >= 0 && point.y <= 1 ? point : null;
}

export function clampToScreen(x: number, y: number, rect: Rect): Point {
  if (rect.width <= 0 || rect.height <= 0) return { x: 0.5, y: 0.5 };
  return { x: clamp01((x - rect.x) / rect.width), y: clamp01((y - rect.y) / rect.height) };
}

export const fromBottom = (point: Point) => point.y >= BOTTOM_EDGE;

export type Fingers = { kind: "one" } | { kind: "pinch" } | { kind: "pan"; offset: Point };

const mirror = (point: Point): Point => ({ x: 1 - point.x, y: 1 - point.y });

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
