//! The Simulator view's Home, Lock and Shake buttons, drawn natively under the screen so they move with it on a resize. The web lays them out and keeps clear buttons over them for clicks, focus and tooltips.

use std::f64::consts::PI;

use objc2::runtime::{AnyObject, Sel};
use objc2::sel;
use objc2_core_foundation::{CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{CGColor, CGMutablePath};
use objc2_foundation::NSString;
use objc2_quartz_core::{
    CACornerMask, CALayer, CAShapeLayer, CATextLayer, kCAAlignmentLeft, kCALineCapRound,
    kCALineJoinRound,
};
use serde::Deserialize;
use specta::Type;

use crate::simulator_kit::{class, msg};

/// The capsule under the screen, measured by the web: CSS pixels, from the capsule's top left.
#[derive(Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenButtons {
    pub width: f64,
    pub height: f64,
    /// From the screen's bottom edge to the capsule's top; the capsule centers under the screen.
    pub gap: f64,
    pub radius: f64,
    pub font_size: f64,
    pub colors: ButtonColors,
    pub faces: Vec<ButtonFace>,
}

/// Opaque sRGB bytes, as the web paints them over the canvas.
#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
pub struct ButtonColors {
    pub ink: [u8; 3],
    pub icon: [u8; 3],
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
    /// None in a narrow view, which shows the icons only.
    pub label: Option<FaceLabel>,
    pub look: FaceLook,
}

#[derive(Deserialize, Type, Clone, Debug, PartialEq)]
pub struct FaceLabel {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The icons of src/ui/icons.tsx the buttons show.
#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FaceIcon {
    Home,
    Lock,
    Unlock,
    Shake,
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

pub(crate) fn srgb([red, green, blue]: [u8; 3]) -> CFRetained<CGColor> {
    let [red, green, blue] = [red, green, blue].map(|part| f64::from(part) / 255.0);
    CGColor::new_srgb(red, green, blue, 1.0)
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
    // The shackle: over the top from the left post, clockwise to `end` radians.
    let shackle = |end: f64| {
        line(&[(5.5, 7.0), (5.5, 5.0)]);
        // SAFETY: as above.
        unsafe { CGMutablePath::add_arc(at, none, 8.0, up(5.0), 2.5, PI, end, true) };
    };
    match icon {
        FaceIcon::Home => {
            line(&[(2.5, 7.25), (8.0, 2.75), (13.5, 7.25)]);
            line(&[(4.0, 6.0), (4.0, 13.25), (12.0, 13.25), (12.0, 6.0)]);
        }
        FaceIcon::Lock => {
            rounded(3.25, 7.0, 9.5, 7.0, 1.75);
            shackle(0.0);
            // SAFETY: as above.
            unsafe { CGMutablePath::add_line_to_point(at, none, 10.5, up(7.0)) };
        }
        // Open: the shackle stops short, at the arc's end point (10.4, 4.3).
        FaceIcon::Unlock => {
            rounded(3.25, 7.0, 9.5, 7.0, 1.75);
            shackle(0.7_f64.atan2(2.4));
        }
        FaceIcon::Shake => {
            rounded(5.25, 2.75, 5.5, 10.5, 1.5);
            line(&[(2.75, 6.0), (2.75, 10.0)]);
            line(&[(13.25, 6.0), (13.25, 10.0)]);
        }
    }
    path
}

/// The system font at `size`, and its line height.
fn system_font(size: f64) -> Option<(*mut AnyObject, f64)> {
    let fonts = class(c"NSFont")?;
    // SAFETY: `systemFontOfSize:` returns an autoreleased NSFont; its metrics return CGFloat.
    unsafe {
        let font = msg::<unsafe extern "C" fn(*mut AnyObject, Sel, f64) -> *mut AnyObject>()(
            fonts,
            sel!(systemFontOfSize:),
            size,
        );
        if font.is_null() {
            return None;
        }
        let metric =
            |selector| msg::<unsafe extern "C" fn(*mut AnyObject, Sel) -> f64>()(font, selector);
        let line = metric(sel!(ascender)) - metric(sel!(descender)) + metric(sel!(leading));
        Some((font, line))
    }
}

/// Fills `capsule` with the buttons of `spec`; `scale` is the window's backing scale. The caller places the capsule.
pub(crate) fn draw(capsule: &CALayer, spec: &ScreenButtons, scale: f64) {
    // SAFETY: drops the old faces.
    unsafe { capsule.setSublayers(None) };
    let colors = spec.colors;
    let height = spec.height;
    capsule.setCornerRadius(spec.radius);
    capsule.setBorderWidth(1.0);
    capsule.setBorderColor(Some(&srgb(colors.line)));
    let font = system_font(spec.font_size);
    let last = spec.faces.len().saturating_sub(1);
    for (index, face) in spec.faces.iter().enumerate() {
        let fill = match face.look {
            FaceLook::Rest => None,
            FaceLook::Hover => Some(colors.hover),
            FaceLook::Pressed => Some(colors.pressed),
        };
        if let Some(fill) = fill {
            let back = CALayer::new();
            back.setFrame(from_top(height, face.x, face.y, face.width, face.height));
            back.setBackgroundColor(Some(&srgb(fill)));
            let left = CACornerMask::LayerMinXMinYCorner | CACornerMask::LayerMinXMaxYCorner;
            let right = CACornerMask::LayerMaxXMinYCorner | CACornerMask::LayerMaxXMaxYCorner;
            let corners = match (index == 0, index == last) {
                (true, true) => left | right,
                (true, false) => left,
                (false, true) => right,
                (false, false) => CACornerMask::empty(),
            };
            back.setMaskedCorners(corners);
            back.setCornerRadius((spec.radius - 1.0).max(0.0));
            capsule.addSublayer(&back);
        }
        if index > 0 {
            let divider = CALayer::new();
            divider.setFrame(from_top(height, face.x, face.y, 1.0, face.height));
            divider.setBackgroundColor(Some(&srgb(colors.line)));
            capsule.addSublayer(&divider);
        }
        let icon = CAShapeLayer::new();
        icon.setFrame(from_top(height, face.icon_x, face.icon_y, ICON, ICON));
        icon.setPath(Some(&icon_path(face.icon)));
        icon.setFillColor(None);
        icon.setStrokeColor(Some(&srgb(colors.icon)));
        icon.setLineWidth(ICON_STROKE);
        // SAFETY: Core Animation's own constants.
        unsafe {
            icon.setLineCap(kCALineCapRound);
            icon.setLineJoin(kCALineJoinRound);
        }
        icon.setContentsScale(scale);
        capsule.addSublayer(&icon);
        if let (Some(label), Some((font, line))) = (&face.label, font) {
            let text = CATextLayer::new();
            // Centered on the web's line box, which is taller than the font's line.
            let top = label.y + (label.height - line) / 2.0;
            // 2px spare: the native text can measure a hair wider than the web's.
            text.setFrame(from_top(height, label.x, top, label.width + 2.0, line));
            // SAFETY: an NSString, and an NSFont the layer retains.
            unsafe {
                text.setString(Some(&NSString::from_str(&label.text)));
                msg::<unsafe extern "C" fn(*const CATextLayer, Sel, *mut AnyObject)>()(
                    &*text,
                    sel!(setFont:),
                    font,
                );
                text.setAlignmentMode(kCAAlignmentLeft);
            }
            text.setFontSize(spec.font_size);
            text.setForegroundColor(Some(&srgb(colors.ink)));
            text.setWrapped(false);
            text.setContentsScale(scale);
            capsule.addSublayer(&text);
        }
    }
}

/// The capsule's frame in a view of `view_height`, under a screen whose top left sits at `screen` in it.
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
