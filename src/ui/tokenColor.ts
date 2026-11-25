export const tokenValue = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(`--${name}`).trim();

let paint: CanvasRenderingContext2D | null | undefined;

export function opaqueRgb(color: string): [number, number, number] {
  paint ??= Object.assign(document.createElement("canvas"), { width: 1, height: 1 }).getContext("2d", { willReadFrequently: true });
  if (!paint) return [0, 0, 0];
  for (const layer of [tokenValue("canvas"), color]) {
    paint.fillStyle = layer;
    paint.fillRect(0, 0, 1, 1);
  }
  const [red, green, blue] = paint.getImageData(0, 0, 1, 1).data;
  return [red, green, blue];
}
