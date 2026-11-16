// The screenshot rows of Settings' Simulators section: a switch for saving the Simulator view's screenshots, off by default, and under it the folder they go in.
import { onOff, Segmented } from "../../../ui/Segmented";
import { Row } from "../../../ui/Row";
import { chooseScreenshotFolder, screenshotFolder, stopSavingScreenshots } from "./store";

export function ScreenshotRows() {
  const folder = screenshotFolder.use();
  return (
    <>
      <Row title="Save screenshots" description={<p className="settings-description">Off: a screenshot lives in a temporary folder, so drag it out to keep it.</p>}>
        <Segmented<boolean> label="Save screenshots in a folder" options={onOff} value={folder !== null} onChange={(on) => (on ? chooseScreenshotFolder() : stopSavingScreenshots())} />
      </Row>
      {folder !== null && (
        <Row nested title="Folder" description={<p className="settings-description settings-path">{folder}</p>}>
          <button type="button" className="button-secondary" onClick={chooseScreenshotFolder}>
            Choose…
          </button>
        </Row>
      )}
    </>
  );
}
