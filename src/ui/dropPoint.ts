// The element under a file drag from the Finder.

// On macOS wry sends the position in CSS points despite its PhysicalPosition type.
export const elementAtDrop = ({ x, y }: { x: number; y: number }) => document.elementFromPoint(x, y);
