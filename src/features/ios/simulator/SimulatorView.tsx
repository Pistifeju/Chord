// The Simulator view in the side panel: Home, Lock, Shake, Screenshot and the Appearance menu under the worktree's simulator screen, which the native view draws through the see-through hole; the screen takes clicks, drags, trackpad pinches and typing. The Preview view shows its preview simulator here with only Screenshot and Appearance. Also its tab bar button.
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { shortcutLabel, tooltip } from "../../../app/commands";
import type { Bezel, Button, ButtonFace, FaceIcon, ScreenButtons } from "../../../ipc/bindings";
import { AppearanceIcon, CameraIcon, HomeIcon, LockIcon, PhoneIcon, ShakeIcon, UnlockIcon } from "../../../ui/icons";
import { seeThrough } from "../../../ui/seeThrough";
import { shortcutChanges } from "../../settings/store";
import { showsSideView, sidePanels, toggleSideView } from "../../sidepanel/store";
import { hidUsage } from "./keys";
import { clampToScreen, cornerRadius, fingersFor, fit, fromBottom, gestureFingers, secondFinger, toScreen, type Fingers, type Point, type Rect } from "./screen";
import { hideScreen, moveScreen, openSimulatorApp, pressButton, screen, sendKey, sendTouch, setButtons, showScreen, takeScreenshot } from "./store";
import { loadAppearance, openAppearanceMenu } from "./appearanceMenu";
import { fail } from "../../worktrees/store";
import { ScreenshotThumbnail } from "./Screenshot";
import "./simulator.css";

export function SimulatorToggle({ worktree }: { worktree: string }) {
  shortcutChanges.use();
  const shown = sidePanels.usePart((all) => showsSideView(all, worktree, "simulator"));
  return (
    <button
      type="button"
      className="icon-button"
      title={tooltip("simulator.toggle")}
      aria-label="Show or Hide Simulator"
      aria-pressed={shown}
      onClick={() => toggleSideView(worktree, "simulator")}
    >
      <PhoneIcon />
    </button>
  );
}

const buttons = (locked: boolean): { button: Button; label: string; face: FaceIcon; icon: ReactNode }[] => [
  { button: "home", label: "Home", face: "home", icon: <HomeIcon /> },
  locked ? { button: "lock", label: "Unlock", face: "unlock", icon: <UnlockIcon /> } : { button: "lock", label: "Lock", face: "lock", icon: <LockIcon /> },
  { button: "shake", label: "Shake", face: "shake", icon: <ShakeIcon /> },
];

type Key = Button | "screenshot" | "appearance";

const boxOf = (element: Element): Rect => {
  const { left, top, width, height } = element.getBoundingClientRect();
  return { x: left, y: top, width, height };
};

/** WebKit's trackpad gesture event. */
type GestureEvent = Event & { scale: number; rotation: number };

type Touch = { fingers: Fingers; fromBottom: boolean };

/** The native bezel's 8px frame and 1px edge (screen_view.rs), the 16px gap and the 28px button row under the screen. */
const BEZEL = 9;
const BELOW_SCREEN = BEZEL + 16 + 28;
/** How far the hole reaches past the bezel, onto the native view's canvas margin (`CANVAS_MARGIN`, screen_view.rs): the native view can run this far ahead on a resize before the web's canvas covers its bezel. */
const HOLE_MARGIN = 32;
/** The area's margin in the view (simulator.css). */
const AREA_MARGIN = 21;

const paint = Object.assign(document.createElement("canvas"), { width: 1, height: 1 }).getContext("2d", { willReadFrequently: true });

/** A color token as opaque sRGB bytes, laid over the canvas as the web paints it. */
function tokenRgb(name: string): [number, number, number] {
  if (!paint) return [0, 0, 0];
  const style = getComputedStyle(document.documentElement);
  for (const token of ["canvas", name]) {
    paint.fillStyle = style.getPropertyValue(`--${token}`).trim();
    paint.fillRect(0, 0, 1, 1);
  }
  const [red, green, blue] = paint.getImageData(0, 0, 1, 1).data;
  return [red, green, blue];
}

const themeColors = () => ({
  bezel: { canvas: tokenRgb("canvas"), frame: tokenRgb("bezel"), edge: tokenRgb("line-strong") },
  buttons: { icon: tokenRgb("ink-secondary"), line: tokenRgb("line-strong"), hover: tokenRgb("raised"), pressed: tokenRgb("selected") },
});

/** The theme's colors for the native bezel and buttons, read again when the theme changes. */
function useThemeColors() {
  const [theme, setTheme] = useState(() => document.documentElement.dataset.theme);
  useEffect(() => {
    const observer = new MutationObserver(() => setTheme(document.documentElement.dataset.theme));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, []);
  return useMemo(themeColors, [theme]);
}

/** The clear buttons' layout and looks, for the native view to draw under the screen. */
function measureButtons(row: HTMLElement, screen: Rect, colors: ScreenButtons["colors"], faces: { face: FaceIcon; look: ButtonFace["look"] }[]): ScreenButtons {
  const box = row.getBoundingClientRect();
  const from = (element: Element) => {
    const { left, top, width, height } = element.getBoundingClientRect();
    return { x: left - box.left, y: top - box.top, width, height };
  };
  return {
    width: box.width,
    height: box.height,
    gap: box.top - (screen.y + screen.height),
    colors,
    outlines: [...row.querySelectorAll(".simulator-capsule")].map(from),
    faces: [...row.querySelectorAll(".simulator-button")].map((element, i) => {
      const icon = from(element.querySelector("svg") ?? element);
      return { icon: faces[i].face, look: faces[i].look, ...from(element), iconX: icon.x, iconY: icon.y };
    }),
  };
}

/** `preview`: the Preview view's line in place of the screen, null once the preview shows; the view then shows the preview simulator with only Screenshot and Appearance. */
export function SimulatorView({ worktree, preview }: { worktree: string; preview?: string | null }) {
  shortcutChanges.use();
  const previewing = preview !== undefined;
  const area = useRef<HTMLDivElement>(null);
  const surface = useRef<HTMLDivElement>(null);
  const [box, setBox] = useState<Rect | null>(null);
  const current = screen.use();
  const state = current?.worktree === worktree ? current.state : null;
  const live = state?.state === "live" && preview == null ? state : null;
  // The screen and its buttons fit together, centered as one group.
  const rect = box && live ? fit({ ...box, height: box.height - BELOW_SCREEN }, live) : box;
  const touch = useRef<Touch | null>(null);
  const hover = useRef<Point | null>(null);
  const keysDown = useRef(new Set<string>());
  const [circles, setCircles] = useState<Point[]>([]);
  const colors = useThemeColors();
  const bezelOf = (screen: Rect): Bezel => ({ radius: cornerRadius(screen), ...colors.bezel });
  const row = useRef<HTMLDivElement>(null);
  const [hovered, setHovered] = useState<Key | null>(null);
  const [pressed, setPressed] = useState<Key | null>(null);
  const sentButtons = useRef("");

  useLayoutEffect(() => {
    const element = area.current;
    if (!element) return;
    const measure = () => setBox((last) => {
      const next = boxOf(element);
      return last && JSON.stringify(last) === JSON.stringify(next) ? last : next;
    });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    window.addEventListener("resize", measure);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, []);

  // A key held while the focus leaves, or while a shortcut hides the view, would stay down on the device.
  const releaseKeys = () => {
    for (const code of keysDown.current) {
      const usage = hidUsage(code);
      if (usage !== null) sendKey(worktree, usage, false);
    }
    keysDown.current.clear();
  };

  const shown = box !== null;
  useEffect(() => {
    if (!shown || !area.current) return;
    const start = boxOf(area.current);
    showScreen(worktree, previewing, start, bezelOf(start));
    return () => {
      releaseKeys();
      hideScreen();
    };
  }, [worktree, previewing, shown]);

  // Read ahead, so the Appearance menu opens without waiting on the simulator.
  useEffect(() => {
    if (live) loadAppearance(worktree, previewing).catch(console.error);
  }, [worktree, live !== null, previewing]);

  const rectKey = rect && JSON.stringify(rect);
  // Layout effects: the hole and the native screen move in the frame React paints them, not one paint later.
  useLayoutEffect(() => {
    if (rect) moveScreen(rect, bezelOf(rect));
  }, [rectKey, colors]);

  // Every render: the lock, hover and press all change the buttons' look. Sent only when it changed.
  useLayoutEffect(() => {
    if (!live || !rect || !row.current) return;
    const keys: { key: Key; face: FaceIcon }[] = [...(previewing ? [] : buttons(live.locked).map(({ button, face }) => ({ key: button, face }))), { key: "screenshot", face: "camera" }, { key: "appearance", face: "appearance" }];
    const faces = keys.map(({ key, face }) => ({ face, look: pressed === key ? ("pressed" as const) : hovered === key ? ("hover" as const) : ("rest" as const) }));
    const next = measureButtons(row.current, rect, colors.buttons, faces);
    const key = JSON.stringify(next);
    if (key !== sentButtons.current) {
      sentButtons.current = key;
      setButtons(next);
    }
  });

  const faceButton = (key: Key, label: string, icon: ReactNode, press: (button: HTMLButtonElement) => void, title = tooltip(`simulator.${key}`)) => (
    <button
      key={key}
      type="button"
      className="simulator-button"
      title={title}
      aria-label={label}
      onClick={(e) => press(e.currentTarget)}
      onPointerEnter={() => setHovered(key)}
      onPointerLeave={() => setHovered((last) => (last === key ? null : last))}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        setPressed(key);
        window.addEventListener("pointerup", () => setPressed(null), { once: true });
      }}
    >
      {icon}
    </button>
  );

  // The hole takes the native bezel and buttons in, and some of the canvas around them.
  useLayoutEffect(() => {
    if (!live || !rect || rect.width === 0) return;
    const outset = BEZEL + HOLE_MARGIN;
    seeThrough.set({ x: rect.x - outset, y: rect.y - outset, width: rect.width + 2 * outset, height: rect.height + outset + BELOW_SCREEN + HOLE_MARGIN, radius: cornerRadius(rect) + outset });
    return () => seeThrough.set(null);
  }, [live !== null, rectKey]);

  const showCircles = (first: Point | null, fingers: Fingers) => {
    const second = first && secondFinger(fingers, first);
    // Hovering without ⌥ keeps the empty list, so a pointer move does not render the view again.
    setCircles((last) => (first && second ? [first, second] : last.length === 0 ? last : []));
  };

  // While ⌥ is held over the screen, the circles show where a press puts the fingers down.
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key !== "Alt" && e.key !== "Shift") return;
      if (!touch.current) showCircles(hover.current, fingersFor({ alt: e.altKey, shift: e.shiftKey }, hover.current ?? { x: 0.5, y: 0.5 }));
    };
    // Capture: the focused screen stops ⌥ from bubbling once it sends it to the device.
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("keyup", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("keyup", onKey, true);
    };
  }, []);

  // A trackpad pinch or rotate: two fingers around the pointer, spread and turned as the gesture goes.
  useEffect(() => {
    const element = surface.current;
    if (!element || !live) return;
    let center: Point | null = null;
    const gesture = (phase: "down" | "move" | "up") => (event: Event) => {
      event.preventDefault();
      const e = event as GestureEvent;
      if (phase === "down") center = hover.current ?? { x: 0.5, y: 0.5 };
      if (!center) return;
      const [first, second] = gestureFingers(center, e.scale, e.rotation);
      sendTouch(worktree, phase, first, second, false);
      if (phase === "up") center = null;
    };
    const start = gesture("down");
    const change = gesture("move");
    const end = gesture("up");
    element.addEventListener("gesturestart", start);
    element.addEventListener("gesturechange", change);
    element.addEventListener("gestureend", end);
    return () => {
      element.removeEventListener("gesturestart", start);
      element.removeEventListener("gesturechange", change);
      element.removeEventListener("gestureend", end);
    };
  }, [worktree, live]);

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    const first = rect && toScreen(e.clientX, e.clientY, rect);
    if (!first || e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    e.currentTarget.focus();
    const fingers = fingersFor({ alt: e.altKey, shift: e.shiftKey }, first);
    touch.current = { fingers, fromBottom: fingers.kind === "one" && fromBottom(first) };
    sendTouch(worktree, "down", first, secondFinger(fingers, first), touch.current.fromBottom);
    showCircles(first, fingers);
  };

  const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
    const pressed = touch.current;
    if (!rect) return;
    if (!pressed) {
      hover.current = toScreen(e.clientX, e.clientY, rect);
      return showCircles(hover.current, fingersFor({ alt: e.altKey, shift: e.shiftKey }, hover.current ?? { x: 0.5, y: 0.5 }));
    }
    const first = clampToScreen(e.clientX, e.clientY, rect);
    sendTouch(worktree, "move", first, secondFinger(pressed.fingers, first), pressed.fromBottom);
    showCircles(first, pressed.fingers);
  };

  const onPointerEnd = (e: PointerEvent<HTMLDivElement>) => {
    const pressed = touch.current;
    if (!pressed || !rect) return;
    touch.current = null;
    const first = clampToScreen(e.clientX, e.clientY, rect);
    sendTouch(worktree, "up", first, secondFinger(pressed.fingers, first), pressed.fromBottom);
    showCircles(toScreen(e.clientX, e.clientY, rect), fingersFor({ alt: e.altKey, shift: e.shiftKey }, first));
  };

  // Keys go to the simulator; ⌘ keys stay the window's shortcuts.
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const usage = hidUsage(e.code);
    if (e.metaKey || usage === null) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.repeat) return;
    keysDown.current.add(e.code);
    sendKey(worktree, usage, true);
  };

  const onKeyUp = (e: KeyboardEvent<HTMLDivElement>) => {
    const usage = hidUsage(e.code);
    if (usage === null || !keysDown.current.delete(e.code)) return;
    e.preventDefault();
    sendKey(worktree, usage, false);
  };


  return (
    <div className="simulator">
      <div className="simulator-area" ref={area}>
        {live && rect && rect.width > 0 && (
          <div ref={row} className="simulator-buttons" role="group" aria-label="Simulator Buttons" style={{ left: rect.x + rect.width / 2, top: rect.y + rect.height + BELOW_SCREEN - 28 }}>
            {!previewing && <div className="simulator-capsule">{buttons(live.locked).map(({ button, label, icon }) => faceButton(button, label, icon, () => pressButton(worktree, button)))}</div>}
            <div className="simulator-capsule simulator-capsule-round">{faceButton("screenshot", "Take Screenshot", <CameraIcon />, () => takeScreenshot(worktree, previewing))}</div>
            <div className="simulator-capsule simulator-capsule-round">
              {faceButton("appearance", "Appearance", <AppearanceIcon />, (button) => void openAppearanceMenu(worktree, previewing, button).catch(fail), "Appearance")}
            </div>
          </div>
        )}
        {live && rect ? (
          <div
            ref={surface}
            className="simulator-screen"
            style={{ left: rect.x, top: rect.y, width: rect.width, height: rect.height, borderRadius: cornerRadius(rect) }}
            role="application"
            aria-label="Simulator Screen"
            tabIndex={0}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerEnd}
            onPointerCancel={onPointerEnd}
            onPointerLeave={() => !touch.current && setCircles([])}
            onKeyDown={onKeyDown}
            onKeyUp={onKeyUp}
            onBlur={releaseKeys}
          >
            {circles.map((point, i) => (
              <span key={i} className="simulator-finger" style={{ left: `${point.x * 100}%`, top: `${point.y * 100}%` }} />
            ))}
          </div>
        ) : (
          <div className="simulator-empty">
            <p>
              {previewing
                ? (preview ?? (state?.state === "unavailable" || state?.state === "off" ? state.message : null) ?? "The preview simulator is off.")
                : state?.state === "unavailable"
                  ? `${state.message} Run (${shortcutLabel("build.run") ?? "⌘R"}) opens ${state.app} instead.`
                  : state?.state === "off" && state.message
                    ? state.message
                    : `The simulator is off. Run (${shortcutLabel("build.run") ?? "⌘R"}) boots it.`}
            </p>
            {state?.state === "unavailable" && !previewing && (
              <button type="button" className="button-secondary" onClick={openSimulatorApp}>
                Open in {state.app}
              </button>
            )}
          </div>
        )}
      </div>
      {live && box && rect && <ScreenshotThumbnail worktree={worktree} bottom={box.y + box.height + AREA_MARGIN - (rect.y + rect.height + BELOW_SCREEN - 28) - 28} />}
    </div>
  );
}
