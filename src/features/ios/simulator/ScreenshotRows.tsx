import { Row } from "../../../ui/Row";
import { chooseScreenshotFolder, screenshotFolder } from "./store";

export function ScreenshotRows() {
  const folder = screenshotFolder.use();
  return (
    <Row title="Save to" description={<p className="settings-description settings-path">{folder ?? "Desktop"}</p>}>
      <button type="button" className="button-secondary" onClick={chooseScreenshotFolder}>
        Choose…
      </button>
    </Row>
  );
}
