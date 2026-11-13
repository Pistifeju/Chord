// Whether a file drag is over the terminal panel, apart from the xterm store so other features skip loading xterm.
import { elementAtDrop } from "../../ui/dropPoint";

export const isOverTerminal = (position: { x: number; y: number }) => !!elementAtDrop(position)?.closest(".terminal-part");
