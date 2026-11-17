// Rules for the Run menu's Scheme and Build Configuration submenus: which scheme and configuration carry the check mark.
import type { SchemePick_Serialize as SchemePick, Schemes } from "../../../ipc/bindings";

/** The pick, else what a build without one uses: the listing's default scheme, and the configuration the shown scheme runs in. */
export function shownPick(pick: SchemePick, listing: Schemes): SchemePick {
  const scheme = pick.scheme ?? listing.default;
  return { scheme, configuration: pick.configuration ?? listing.schemes.find((listed) => listed.name === scheme)?.configuration ?? null };
}
