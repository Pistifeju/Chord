// Rules for the Run menu's Scheme and Build Configuration submenus: which scheme and configuration carry the check mark, and which the toolbar shows before the listing comes.
import type { SchemePick_Serialize as SchemePick, Schemes } from "../../../ipc/bindings";

/** The pick, else what a build without one uses: the listing's default scheme, and the configuration the shown scheme runs in. */
export function shownPick(pick: SchemePick, listing: Schemes): SchemePick {
  const scheme = pick.scheme ?? listing.default;
  return { scheme, configuration: pick.configuration ?? listing.schemes.find((listed) => listed.name === scheme)?.configuration ?? null };
}

/** Before the listing comes: the pick over the defaults the last listing gave; a picked scheme other than the default shows no configuration yet. */
export function shownBeforeListing(pick: SchemePick, defaults: SchemePick | undefined): SchemePick {
  if (!defaults) return pick;
  const scheme = pick.scheme ?? defaults.scheme;
  return { scheme, configuration: pick.configuration ?? (scheme === defaults.scheme ? defaults.configuration : null) };
}
