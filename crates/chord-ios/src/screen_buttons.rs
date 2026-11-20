//! The Simulator view's button rail (Home, Lock and Shake, then screenshot and record, then Appearance, split by hairlines), the record button grown into a pill with the recording's time, drawn natively under the screen so they move with it on a resize, with the lock's shackle turning and the phone shaking. The web lays them out and keeps clear buttons over them for clicks, focus and tooltips.

use std::f64::consts::PI;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSFont, NSFontWeightMedium};
use objc2_core_foundation::{CFRetained, CFType, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{CGColor, CGMutablePath};
use objc2_foundation::{NSArray, NSNumber, NSString};
use objc2_quartz_core::{
    CAAnimation, CABasicAnimation, CAKeyframeAnimation, CALayer, CAMediaTiming,
    CAMediaTimingFunction, CAShapeLayer, CATextLayer, CATransform3D, kCAAlignmentLeft,
    kCALineCapRound, kCALineJoinRound, kCAMediaTimingFunctionEaseOut,
};
use serde::Deserialize;
use specta::Type;

/// The button row under the screen, measured by the web: CSS pixels, from the row's top left.
#[derive(Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenButtons {
    pub width: f64,
    pub height: f64,
    /// From the screen's bottom edge to the row's top; the row centers under the screen.
    pub gap: f64,
    pub colors: ButtonColors,
    /// The hairline pills the faces sit in: the rail, or each group once a narrow view wraps it.
    pub outlines: Vec<Outline>,
    /// The hairlines between the rail's groups.
    pub dividers: Vec<Outline>,
    pub faces: Vec<ButtonFace>,
    /// The recording's time, while the record pill shows.
    pub timer: Option<Timer>,
}

/// `text` at its box, cut to `clip`, the part of the pill it shows in.
#[derive(Deserialize, Type, Clone, Debug, PartialEq)]
pub struct Timer {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub clip: Outline,
}

#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Opaque sRGB bytes, as the web paints them over the canvas.
#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
pub struct ButtonColors {
    pub icon: [u8; 3],
    pub text: [u8; 3],
    pub danger: [u8; 3],
    pub line: [u8; 3],
    pub hover: [u8; 3],
    pub pressed: [u8; 3],
}

#[derive(Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ButtonFace {
    pub icon: FaceIcon,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// The 16px icon's top left.
    pub icon_x: f64,
    pub icon_y: f64,
    pub look: FaceLook,
}

/// The icons of src/ui/icons.tsx the buttons show.
#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FaceIcon {
    Home,
    Lock,
    Unlock,
    Shake,
    Camera,
    Record,
    Stop,
    Appearance,
}

#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FaceLook {
    Rest,
    Hover,
    Pressed,
}

const ICON: f64 = 16.0;
const ICON_STROKE: f64 = 1.3;

/// When the lock icon last turned and the phone last shook, in media time. Every `draw` adds what is left of each, so a redraw for a hover keeps them going.
#[derive(Default, Clone, Copy)]
pub(crate) struct Motion {
    pub lock: Option<f64>,
    pub shake: Option<f64>,
}

/// The Lock or Unlock icon `spec` shows.
pub(crate) fn lock_icon(spec: &ScreenButtons) -> Option<FaceIcon> {
    spec.faces
        .iter()
        .map(|face| face.icon)
        .find(|icon| matches!(icon, FaceIcon::Lock | FaceIcon::Unlock))
}

pub(crate) fn srgb([red, green, blue]: [u8; 3]) -> CFRetained<CGColor> {
    let [red, green, blue] = [red, green, blue].map(|part| f64::from(part) / 255.0);
    CGColor::new_srgb(red, green, blue, 1.0)
}

fn srgb_alpha([red, green, blue]: [u8; 3], alpha: f64) -> CFRetained<CGColor> {
    let [red, green, blue] = [red, green, blue].map(|part| f64::from(part) / 255.0);
    CGColor::new_srgb(red, green, blue, alpha)
}

/// A box measured from the top left, in a layer whose origin is its bottom left.
fn from_top(height: f64, x: f64, y: f64, width: f64, box_height: f64) -> CGRect {
    CGRect::new(
        CGPoint::new(x, height - y - box_height),
        CGSize::new(width, box_height),
    )
}

/// `icon` as in src/ui/icons.tsx, its 16px box turned so y grows up.
fn icon_path(icon: FaceIcon) -> CFRetained<CGMutablePath> {
    let path = CGMutablePath::new();
    let at = Some(&*path);
    let none = std::ptr::null();
    let up = |y: f64| ICON - y;
    let line = |points: &[(f64, f64)]| {
        let mut points = points.iter();
        if let Some((x, y)) = points.next() {
            // SAFETY: a live path and no transform.
            unsafe { CGMutablePath::move_to_point(at, none, *x, up(*y)) };
        }
        for (x, y) in points {
            // SAFETY: as above.
            unsafe { CGMutablePath::add_line_to_point(at, none, *x, up(*y)) };
        }
    };
    let rounded = |x: f64, y: f64, width: f64, height: f64, radius: f64| {
        let rect = CGRect::new(CGPoint::new(x, up(y + height)), CGSize::new(width, height));
        // SAFETY: as above.
        unsafe { CGMutablePath::add_rounded_rect(at, none, rect, radius, radius) };
    };
    match icon {
        FaceIcon::Home => {
            line(&[(2.5, 7.25), (8.0, 2.75), (13.5, 7.25)]);
            line(&[(4.0, 6.0), (4.0, 13.25), (12.0, 13.25), (12.0, 6.0)]);
        }
        // The body; the shackle is its own layer (`shackle_path`).
        FaceIcon::Lock | FaceIcon::Unlock => rounded(3.25, 7.0, 9.5, 7.0, 1.75),
        FaceIcon::Shake => {
            rounded(5.25, 2.75, 5.5, 10.5, 1.5);
            line(&[(2.75, 6.0), (2.75, 10.0)]);
            line(&[(13.25, 6.0), (13.25, 10.0)]);
        }
        FaceIcon::Camera => {
            line(&[
                (2.5, 5.25),
                (4.6, 5.25),
                (5.6, 3.75),
                (10.4, 3.75),
                (11.4, 5.25),
                (13.5, 5.25),
                (13.5, 12.75),
                (2.5, 12.75),
                (2.5, 5.25),
            ]);
            line(&[(10.25, 8.75)]);
            // SAFETY: as above.
            unsafe { CGMutablePath::add_arc(at, none, 8.0, up(8.75), 2.25, 0.0, 2.0 * PI, false) };
        }
        FaceIcon::Record => {
            // SAFETY: as above.
            unsafe { CGMutablePath::add_arc(at, none, 8.0, 8.0, 4.5, 0.0, 2.0 * PI, false) };
        }
        FaceIcon::Stop => rounded(4.0, 4.0, 8.0, 8.0, 1.5),
        // The ring; the filled half is its own layer (`left_half_path`).
        FaceIcon::Appearance => {
            // SAFETY: as above.
            unsafe { CGMutablePath::add_arc(at, none, 8.0, 8.0, 6.0, 0.0, 2.0 * PI, false) };
        }
    }
    path
}

/// The filled left half of src/ui/icons.tsx's appearance icon.
fn left_half_path() -> CFRetained<CGMutablePath> {
    let path = CGMutablePath::new();
    let at = Some(&*path);
    let none = std::ptr::null();
    // SAFETY: a live path and no transform.
    unsafe {
        CGMutablePath::add_arc(at, none, 8.0, 8.0, 6.0, PI / 2.0, 3.0 * PI / 2.0, false);
        CGMutablePath::close_subpath(at);
    }
    path
}

/// The closed shackle of src/ui/icons.tsx's lock: up the left post, over the top, down the right post.
fn shackle_path() -> CFRetained<CGMutablePath> {
    let path = CGMutablePath::new();
    let at = Some(&*path);
    let none = std::ptr::null();
    // SAFETY: a live path and no transform.
    unsafe {
        CGMutablePath::move_to_point(at, none, 5.5, ICON - 7.0);
        CGMutablePath::add_line_to_point(at, none, 5.5, ICON - 5.0);
        CGMutablePath::add_arc(at, none, 8.0, ICON - 5.0, 2.5, PI, 0.0, true);
        CGMutablePath::add_line_to_point(at, none, 10.5, ICON - 7.0);
    }
    path
}

/// The share of the closed shackle the open one draws: its left post and the arc to (10.4, 4.3).
fn open_shackle() -> f64 {
    (2.0 + 2.5 * (PI - 0.7_f64.atan2(2.4))) / (4.0 + 2.5 * PI)
}

fn number(value: f64) -> Retained<NSNumber> {
    NSNumber::new_f64(value)
}

fn ease_out() -> Retained<CAMediaTimingFunction> {
    // SAFETY: Core Animation's own constant.
    CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseOut })
}

/// A turn of `layer` about its center through `angles` (radians), from `begin` for `duration` seconds.
fn turn(layer: &CALayer, angles: &[f64], begin: f64, duration: f64) {
    let values: Vec<Retained<NSNumber>> = angles.iter().copied().map(number).collect();
    let values: Vec<&AnyObject> = values.iter().map(|value| -> &AnyObject { value }).collect();
    let animation = CAKeyframeAnimation::animationWithKeyPath(Some(&NSString::from_str(
        "transform.rotation.z",
    )));
    // SAFETY: an array of NSNumber, as the key path takes.
    unsafe { animation.setValues(Some(&NSArray::from_slice(&values))) };
    animation.setBeginTime(begin);
    animation.setDuration(duration);
    animation.setTimingFunction(Some(&ease_out()));
    layer.addAnimation_forKey(&animation, Some(&NSString::from_str("turn")));
}

/// The shackle's stroke from `from` to the layer's own end, from `begin`.
fn draw_shackle(shackle: &CAShapeLayer, from: f64, begin: f64) {
    let animation = CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str("strokeEnd")));
    let from = number(from);
    let from: &AnyObject = &from;
    // SAFETY: an NSNumber, as strokeEnd takes.
    unsafe { animation.setFromValue(Some(from)) };
    animation.setBeginTime(begin);
    animation.setDuration(0.3);
    animation.setTimingFunction(Some(&ease_out()));
    let animation: &CAAnimation = &animation;
    shackle.addAnimation_forKey(animation, Some(&NSString::from_str("shackle")));
}

/// A 16px icon stroke in `color`.
fn stroke(path: &CGMutablePath, color: &CGColor, scale: f64) -> Retained<CAShapeLayer> {
    let layer = CAShapeLayer::new();
    layer.setPath(Some(path));
    layer.setFillColor(None);
    layer.setStrokeColor(Some(color));
    layer.setLineWidth(ICON_STROKE);
    // SAFETY: Core Animation's own constants.
    unsafe {
        layer.setLineCap(kCALineCapRound);
        layer.setLineJoin(kCALineJoinRound);
    }
    layer.setContentsScale(scale);
    layer
}

/// A pill: a radius of half its height.
fn pill(layer: &CALayer, frame: CGRect) {
    layer.setFrame(frame);
    layer.setCornerRadius(frame.size.height / 2.0);
}

/// Fills `row` with the outlines and faces of `spec`; `scale` is the window's backing scale. The caller places the row.
pub(crate) fn draw(row: &CALayer, spec: &ScreenButtons, scale: f64, motion: Motion) {
    // SAFETY: drops the old faces.
    unsafe { row.setSublayers(None) };
    let colors = spec.colors;
    let height = spec.height;
    for outline in &spec.outlines {
        let edge = CALayer::new();
        pill(
            &edge,
            from_top(height, outline.x, outline.y, outline.width, outline.height),
        );
        edge.setBorderWidth(1.0);
        edge.setBorderColor(Some(&srgb(colors.line)));
        row.addSublayer(&edge);
    }
    for divider in &spec.dividers {
        let line = CALayer::new();
        line.setFrame(from_top(
            height,
            divider.x,
            divider.y,
            divider.width,
            divider.height,
        ));
        line.setBackgroundColor(Some(&srgb(colors.line)));
        row.addSublayer(&line);
    }
    if let Some(timer) = &spec.timer {
        row.addSublayer(&timer_layer(timer, height, colors.text, scale));
    }
    for face in &spec.faces {
        // The stop button sits on a red wash that deepens on hover and press.
        let fill = match (face.icon, face.look) {
            (FaceIcon::Stop, look) => Some(srgb_alpha(
                colors.danger,
                match look {
                    FaceLook::Rest => 0.18,
                    FaceLook::Hover => 0.26,
                    FaceLook::Pressed => 0.34,
                },
            )),
            (_, FaceLook::Rest) => None,
            (_, FaceLook::Hover) => Some(srgb(colors.hover)),
            (_, FaceLook::Pressed) => Some(srgb(colors.pressed)),
        };
        if let Some(fill) = fill {
            let back = CALayer::new();
            pill(
                &back,
                from_top(height, face.x, face.y, face.width, face.height),
            );
            back.setBackgroundColor(Some(&fill));
            row.addSublayer(&back);
        }
        let color = srgb(match face.icon {
            FaceIcon::Stop => colors.danger,
            _ => colors.icon,
        });
        let icon = stroke(&icon_path(face.icon), &color, scale);
        icon.setFrame(from_top(height, face.icon_x, face.icon_y, ICON, ICON));
        // A pressed icon shrinks to 90%, as the web's icon buttons do.
        if face.look == FaceLook::Pressed {
            icon.setTransform(CATransform3D::new_scale(0.9, 0.9, 1.0));
        }
        match face.icon {
            FaceIcon::Lock | FaceIcon::Unlock => {
                let shackle = stroke(&shackle_path(), &color, scale);
                shackle.setFrame(CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(ICON, ICON)));
                let (open, closed) = (open_shackle(), 1.0);
                let (from, to) = match face.icon {
                    FaceIcon::Lock => (open, closed),
                    _ => (closed, open),
                };
                shackle.setStrokeEnd(to);
                if let Some(begin) = motion.lock {
                    draw_shackle(&shackle, from, begin);
                    turn(&icon, &[0.0, -0.14, 0.07, 0.0], begin, 0.35);
                }
                icon.addSublayer(&shackle);
            }
            FaceIcon::Shake => {
                if let Some(begin) = motion.shake {
                    turn(
                        &icon,
                        &[0.0, -0.26, 0.22, -0.16, 0.1, -0.04, 0.0],
                        begin,
                        0.45,
                    );
                }
            }
            FaceIcon::Appearance => {
                let half = stroke(&left_half_path(), &color, scale);
                half.setFillColor(Some(&color));
                half.setFrame(CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(ICON, ICON)));
                icon.addSublayer(&half);
            }
            FaceIcon::Record | FaceIcon::Stop => icon.setFillColor(Some(&color)),
            FaceIcon::Home | FaceIcon::Camera => {}
        }
        row.addSublayer(&icon);
    }
}

/// The timer's text in a layer cut to its box, so the text shows as the pill grows.
fn timer_layer(timer: &Timer, height: f64, color: [u8; 3], scale: f64) -> Retained<CALayer> {
    let clip = CALayer::new();
    let Outline {
        x,
        y,
        width,
        height: clip_height,
    } = timer.clip;
    clip.setFrame(from_top(height, x, y, width, clip_height));
    clip.setMasksToBounds(true);
    let text = CATextLayer::new();
    let size = 13.0;
    // Centered on the web's text box: Core Text puts the line's top at the frame's top.
    let line = (size * 1.25_f64).ceil();
    text.setFrame(from_top(
        clip_height,
        timer.x - x,
        timer.y - y + (timer.height - line) / 2.0,
        timer.width.max(1.0) + 8.0,
        line,
    ));
    // SAFETY: the system font always exists and its weight constant is AppKit's own.
    let font = NSFont::monospacedDigitSystemFontOfSize_weight(size, unsafe { NSFontWeightMedium });
    let string = NSString::from_str(&timer.text);
    let string: &AnyObject = &string;
    // SAFETY: NSFont is toll-free bridged with CTFont, a font CATextLayer takes; a string as `string` takes.
    unsafe {
        text.setFont(Some(&*(Retained::as_ptr(&font) as *const CFType)));
        text.setString(Some(string));
    }
    text.setFontSize(size);
    text.setForegroundColor(Some(&srgb(color)));
    // SAFETY: Core Animation's own constant.
    text.setAlignmentMode(unsafe { kCAAlignmentLeft });
    text.setContentsScale(scale);
    clip.addSublayer(&text);
    clip
}

/// The row's frame in a view of `view_height`, under a screen whose top left sits at `screen` in it.
pub(crate) fn capsule_frame(
    spec: &ScreenButtons,
    view_height: f64,
    screen: CGPoint,
    screen_size: CGSize,
) -> CGRect {
    from_top(
        view_height,
        screen.x + (screen_size.width - spec.width) / 2.0,
        screen.y + screen_size.height + spec.gap,
        spec.width,
        spec.height,
    )
}
