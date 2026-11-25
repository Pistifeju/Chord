import type { SchemePick_Serialize as SchemePick, Schemes } from "../../../ipc/bindings";

export function shownPick(pick: SchemePick, listing: Schemes): SchemePick {
  const scheme = pick.scheme ?? listing.default;
  return { scheme, configuration: pick.configuration ?? listing.schemes.find((listed) => listed.name === scheme)?.configuration ?? null };
}

export function shownBeforeListing(pick: SchemePick, defaults: SchemePick | undefined): SchemePick {
  if (!defaults) return pick;
  const scheme = pick.scheme ?? defaults.scheme;
  return { scheme, configuration: pick.configuration ?? (scheme === defaults.scheme ? defaults.configuration : null) };
}
