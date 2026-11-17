// The alert the toast shows: a failure, which stays until dismissed, or a notice, which closes itself.
import { externalStore } from "./externalStore";

export type Alert = { text: string; notice: boolean };

export const alert = externalStore<Alert | null>(null);

export const fail = (error: unknown) => alert.set({ text: String(error), notice: false });
export const notify = (text: string) => alert.set({ text, notice: true });
export const dismissAlert = () => alert.set(null);
