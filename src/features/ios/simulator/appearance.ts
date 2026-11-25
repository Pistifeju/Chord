import type { SimulatorAppearance, SimulatorAppearanceChange } from "../../../ipc/bindings";

export const sizeLabel = (category: string) =>
  category
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");

export function namedCodes(codes: string[], nameOf: (code: string) => string | undefined): { code: string; name: string }[] {
  return codes.map((code) => ({ code, name: nameOf(code) ?? code })).sort((a, b) => a.name.localeCompare(b.name));
}

export function withChange(now: SimulatorAppearance, change: SimulatorAppearanceChange): SimulatorAppearance {
  switch (change.kind) {
    case "style":
      return { ...now, style: change.style };
    case "content_size":
      return { ...now, contentSize: change.size };
    case "language":
      return { ...now, language: change.language };
    case "region":
      return { ...now, region: change.region };
  }
}
