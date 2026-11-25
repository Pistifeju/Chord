import { elementAtDrop } from "../../ui/dropPoint";

export const isOverTerminal = (position: { x: number; y: number }) => !!elementAtDrop(position)?.closest(".terminal-part");
