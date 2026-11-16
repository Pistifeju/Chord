// The see-through hole: where the window's background opens, so the native view under the webview shows there.
import { externalStore } from "./externalStore";

export type Hole = { x: number; y: number; width: number; height: number; radius: number };

export const seeThrough = externalStore<Hole | null>(null);
