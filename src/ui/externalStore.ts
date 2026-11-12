// A value outside React that views read with useSyncExternalStore.
import { useSyncExternalStore } from "react";

export function externalStore<T>(initial: T) {
  let value = initial;
  const listeners = new Set<() => void>();
  const subscribe = (listener: () => void) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  };
  const get = () => value;
  return {
    get,
    subscribe,
    set(next: T) {
      value = next;
      listeners.forEach((listener) => listener());
    },
    use: () => useSyncExternalStore(subscribe, get),
    // The view renders again only when its part changes; `part` must return the same value for the same store value.
    usePart: <P>(part: (value: T) => P) => useSyncExternalStore(subscribe, () => part(value)),
  };
}
