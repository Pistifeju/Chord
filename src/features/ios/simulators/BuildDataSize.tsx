// The size of the worktrees' build data, as the value of Settings' Build Data row.
import { useEffect } from "react";
import { sizeText } from "./simulators";
import { buildDataSize, loadBuildDataSize } from "./store";

export function BuildDataSize() {
  useEffect(() => {
    loadBuildDataSize();
  }, []);
  const bytes = buildDataSize.use();
  return <span className="settings-size">{bytes === null ? "" : sizeText(bytes)}</span>;
}
