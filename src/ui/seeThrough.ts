import { externalStore } from "./externalStore";

type Hole = { x: number; y: number; width: number; height: number; radius: number };

export const seeThrough = externalStore<Hole | null>(null);
