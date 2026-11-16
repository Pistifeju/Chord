//! Input for the simulator the Simulator view shows: the view's touches, keys and buttons, over DTUHID from CoreSimulator 1155.4 (Xcode 27) on and over Indigo before, with the rule that keeps a touch's moves in order.

mod dtuhid;
mod indigo;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use serde::Deserialize;
use chord_core::Error;
use specta::Type;

use crate::simulator;

/// The first CoreSimulator whose guest takes DTUHID input; the Indigo keyboard and lock stop working there.
const DTUHID_FROM: [u32; 2] = [1155, 4];
/// The edge flag that makes a swipe from the bottom a system swipe (home on Face ID models), on both channels.
const EDGE_BOTTOM: u32 = 3;

/// A point on the screen: 0…1 from the top left.
#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TouchPhase {
    Down,
    Move,
    Up,
}

#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    Home,
    Lock,
    Shake,
}

#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum Input {
    /// One or two fingers. `seq` grows with every touch the view sends; `from_bottom` marks a swipe that starts at the bottom edge.
    Touch {
        seq: u32,
        phase: TouchPhase,
        first: Point,
        second: Option<Point>,
        from_bottom: bool,
    },
    /// A USB HID keyboard usage (page 7).
    Key {
        usage: u32,
        down: bool,
    },
    Button {
        button: Button,
    },
}

/// Input that runs a tool; run it off the main thread.
pub struct Blocking(Box<dyn FnOnce() -> Result<(), Error> + Send>);

impl Blocking {
    pub fn run(self) -> Result<(), Error> {
        (self.0)()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Channel {
    Dtuhid,
    Indigo,
}

fn channel_for(version: &[u32]) -> Channel {
    if version >= &DTUHID_FROM[..] {
        Channel::Dtuhid
    } else {
        Channel::Indigo
    }
}

/// The newest touch a device got. The view's touches can arrive out of order over IPC.
#[derive(Default)]
struct TouchOrder {
    last: Option<u32>,
}

impl TouchOrder {
    /// Whether the touch goes to the device: a move older than the newest touch is dropped; a down or an up always goes, and a down starts the count again, so a reloaded view's touches pass.
    fn admits(&mut self, seq: u32, phase: TouchPhase) -> bool {
        let newest = phase == TouchPhase::Down || self.last.is_none_or(|last| seq > last);
        if newest {
            self.last = Some(seq);
        }
        newest || phase == TouchPhase::Up
    }
}

enum Link {
    Dtuhid(dtuhid::Link),
    Indigo(indigo::Client),
}

/// The input link to one booted device, kept for as long as it stays booted.
pub(crate) struct Hid {
    udid: String,
    link: Link,
    order: TouchOrder,
}

impl Hid {
    /// Main thread. None when this CoreSimulator offers no link to the device.
    pub fn new(version: &[u32], udid: String, device: &Retained<AnyObject>) -> Option<Hid> {
        let link = match channel_for(version) {
            Channel::Dtuhid => Link::Dtuhid(dtuhid::Link::connect(device)),
            Channel::Indigo => Link::Indigo(indigo::Client::new(device)?),
        };
        Some(Hid {
            udid,
            link,
            order: TouchOrder::default(),
        })
    }

    /// Main thread. Sends `input` at once, or returns the tool run that carries it.
    pub fn send(&mut self, input: Input) -> Option<Blocking> {
        let udid = || self.udid.clone();
        match (input, &self.link) {
            (
                Input::Button {
                    button: Button::Shake,
                },
                _,
            ) => {
                let udid = udid();
                Some(Blocking(Box::new(move || simulator::shake(&udid))))
            }
            (
                Input::Button {
                    button: Button::Home,
                },
                Link::Indigo(_),
            ) => {
                let udid = udid();
                Some(Blocking(Box::new(move || {
                    simulator::show_home_screen(&udid)
                })))
            }
            (Input::Touch { seq, phase, .. }, _) if !self.order.admits(seq, phase) => None,
            (input, Link::Dtuhid(link)) => {
                link.send(input);
                None
            }
            (input, Link::Indigo(client)) => {
                client.send(input);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_dtuhid_from_core_simulator_1155_4() {
        assert_eq!(channel_for(&[1174, 9, 2]), Channel::Dtuhid);
        assert_eq!(channel_for(&[1155, 4]), Channel::Dtuhid);
        assert_eq!(channel_for(&[1155, 4, 1]), Channel::Dtuhid);
        assert_eq!(channel_for(&[1155, 3, 9]), Channel::Indigo);
        assert_eq!(channel_for(&[1051, 17, 7]), Channel::Indigo);
        assert_eq!(channel_for(&[]), Channel::Indigo);
    }

    #[test]
    fn drops_only_the_moves_older_than_the_newest_touch() {
        let mut order = TouchOrder::default();
        assert!(order.admits(1, TouchPhase::Down));
        assert!(order.admits(3, TouchPhase::Move));
        assert!(!order.admits(2, TouchPhase::Move));
        assert!(!order.admits(3, TouchPhase::Move));
        assert!(order.admits(5, TouchPhase::Up));
        assert!(order.admits(4, TouchPhase::Up));
        assert!(!order.admits(4, TouchPhase::Move));
        assert!(order.admits(6, TouchPhase::Move));
    }

    #[test]
    fn a_down_starts_the_count_again() {
        let mut order = TouchOrder::default();
        assert!(order.admits(900, TouchPhase::Move));
        assert!(order.admits(1, TouchPhase::Down));
        assert!(order.admits(2, TouchPhase::Move));
        assert!(!order.admits(1, TouchPhase::Move));
    }
}
