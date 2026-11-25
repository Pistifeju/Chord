import AppKit
import DeveloperToolsSupport
import QuartzCore
import SwiftUI

enum Body {
    case swiftUI(@MainActor () -> any View)
    case controller(@MainActor () -> NSViewController)
    case view(@MainActor () -> NSView)
}

typealias ControllerInit = @convention(thin) (String?, Traits, @escaping @MainActor () -> NSViewController) -> Preview
typealias ViewInit = @convention(thin) (String?, Traits, @escaping @MainActor () -> NSView) -> Preview

@_silgen_name("chord_controller_init")
@MainActor func controllerInit(_ name: String?, traits: Traits, body: @escaping @MainActor () -> NSViewController) -> Preview {
    captured = Captured(name: name, traits: traits, body: .controller(body))
    return unsafeBitCast(unsafeBitCast(interposeController.1, to: UnsafeRawPointer.self), to: ControllerInit.self)(name, traits, body)
}

@_silgen_name("chord_view_init")
@MainActor func viewInit(_ name: String?, traits: Traits, body: @escaping @MainActor () -> NSView) -> Preview {
    captured = Captured(name: name, traits: traits, body: .view(body))
    return unsafeBitCast(unsafeBitCast(interposeView.1, to: UnsafeRawPointer.self), to: ViewInit.self)(name, traits, body)
}

@_extern(c, "chord_controller_init_alias") func controllerInitC()
@_extern(c, "chord_view_init_alias") func viewInitC()
@_extern(c, "$s21DeveloperToolsSupport7PreviewV6AppKitE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdSo16NSViewControllerCyScMYcctcfC") func originalControllerInit()
@_extern(c, "$s21DeveloperToolsSupport7PreviewV6AppKitE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdSo6NSViewCyScMYcctcfC") func originalViewInit()

@section("__DATA,__interpose") @used
let interposeController: (@convention(c) () -> Void, @convention(c) () -> Void) = (controllerInitC, originalControllerInit)
@section("__DATA,__interpose") @used
let interposeView: (@convention(c) () -> Void, @convention(c) () -> Void) = (viewInitC, originalViewInit)

@MainActor func controller(for c: Captured, fixed: Bool) -> NSViewController {
    switch c.body {
    case .controller(let make):
        return make()
    case .view(let make):
        let box = NSViewController()
        box.view = make()
        return box
    case .swiftUI(let make):
        let host = NSHostingController(rootView: AnyView(make()))
        if fixed { host.sizingOptions = [] }
        return host
    }
}

final class PreviewWindow: NSWindow {
    @objc func hasKeyAppearance() -> Bool { true }
}

@MainActor enum Screen {
    static let window: NSWindow = {
        let w = PreviewWindow(
            contentRect: NSRect(x: 0, y: 0, width: 480, height: 320),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        w.isReleasedWhenClosed = false
        w.alphaValue = 0
        w.ignoresMouseEvents = true
        if let screen = NSScreen.main { w.setFrameOrigin(screen.visibleFrame.origin) }
        w.orderFrontRegardless()
        return w
    }()

    static func contextID() -> UInt32? {
        guard let type = NSClassFromString("CAContext") as? NSObject.Type,
              let all = type.perform(NSSelectorFromString("allContexts"))?.takeUnretainedValue() as? [NSObject],
              let context = all.first(where: { ($0.value(forKey: "layer") as? CALayer)?.name == "NSCGSWindow" }),
              let id = context.value(forKey: "contextId") as? NSNumber else { return nil }
        return id.uint32Value
    }

    static func orderIn() {
        if NSApp.isHidden { NSApp.unhideWithoutActivation() }
        if !window.isVisible { window.orderFrontRegardless() }
    }

    static func present(_ c: Captured) -> [String: Any] {
        orderIn()
        let layout = layout(of: c.traits)
        let fixed: CGSize? = if case .fixed(let s) = layout { s } else { nil }
        let content = controller(for: c, fixed: fixed != nil)
        window.contentViewController = content
        window.title = c.name ?? ""
        let fitting = content.view.fittingSize
        if let size = fixed ?? (fitting.width > 0 && fitting.height > 0 ? fitting : nil) {
            window.setContentSize(size)
        }
        return [
            "ok": true,
            "size": [window.frame.width, window.frame.height],
            "scale": window.backingScaleFactor,
        ]
    }

    static var click = 0
    static var modifiers: CGEventFlags = []

    static func input(_ request: [String: Any]) -> [String: Any] {
        orderIn()
        let x = request["x"] as? Double ?? 0
        let point = NSPoint(x: x, y: window.frame.height - (request["y"] as? Double ?? 0))
        let now = ProcessInfo.processInfo.systemUptime
        switch request["kind"] as? String {
        case "mouse":
            let phase = request["phase"] as? String
            let type: NSEvent.EventType = switch phase {
            case "down": .leftMouseDown
            case "up": .leftMouseUp
            default: .leftMouseDragged
            }
            if phase == "down" { click += 1 }
            guard let event = NSEvent.mouseEvent(
                with: type, location: point, modifierFlags: [], timestamp: now,
                windowNumber: window.windowNumber, context: nil, eventNumber: click,
                clickCount: 1, pressure: phase == "up" ? 0 : 1
            ) else { return ["error": "No mouse event"] }
            window.sendEvent(event)
        case "scroll":
            let dx = request["dx"] as? Double ?? 0
            let dy = request["dy"] as? Double ?? 0
            guard let scroll = CGEvent(
                scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 2,
                wheel1: Int32((-dy).rounded()), wheel2: Int32((-dx).rounded()), wheel3: 0
            ) else { return ["error": "No scroll event"] }
            scroll.location = CGPoint(x: point.x, y: (NSScreen.screens.first?.frame.height ?? 0) - point.y)
            guard let event = NSEvent(cgEvent: scroll) else { return ["error": "No scroll event"] }
            window.sendEvent(event)
        case "key":
            guard let usage = request["usage"] as? Int else { return ["ok": true] }
            let down = request["down"] as? Bool ?? false
            if let flag = modifierFlags[usage] {
                if down { modifiers.insert(flag) } else { modifiers.remove(flag) }
                return ["ok": true]
            }
            guard let code = keyCodes[usage] else { return ["ok": true] }
            guard let key = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else { return ["error": "No key event"] }
            key.flags = modifiers
            guard let typed = NSEvent(cgEvent: key), let event = NSEvent.keyEvent(
                with: typed.type, location: .zero, modifierFlags: typed.modifierFlags, timestamp: now,
                windowNumber: window.windowNumber, context: nil, characters: typed.characters ?? "",
                charactersIgnoringModifiers: typed.charactersIgnoringModifiers ?? "", isARepeat: false, keyCode: code
            ) else { return ["error": "No key event"] }
            window.sendEvent(event)
        default:
            return ["error": "Unknown input"]
        }
        return ["ok": true]
    }
}

let modifierFlags: [Int: CGEventFlags] = [
    0xe0: .maskControl, 0xe1: .maskShift, 0xe2: .maskAlternate,
    0xe4: .maskControl, 0xe5: .maskShift, 0xe6: .maskAlternate,
]

let keyCodes: [Int: CGKeyCode] = [
    0x04: 0x00, 0x05: 0x0B, 0x06: 0x08, 0x07: 0x02, 0x08: 0x0E, 0x09: 0x03, 0x0A: 0x05, 0x0B: 0x04,
    0x0C: 0x22, 0x0D: 0x26, 0x0E: 0x28, 0x0F: 0x25, 0x10: 0x2E, 0x11: 0x2D, 0x12: 0x1F, 0x13: 0x23,
    0x14: 0x0C, 0x15: 0x0F, 0x16: 0x01, 0x17: 0x11, 0x18: 0x20, 0x19: 0x09, 0x1A: 0x0D, 0x1B: 0x07,
    0x1C: 0x10, 0x1D: 0x06,
    0x1E: 0x12, 0x1F: 0x13, 0x20: 0x14, 0x21: 0x15, 0x22: 0x17, 0x23: 0x16, 0x24: 0x1A, 0x25: 0x1C,
    0x26: 0x19, 0x27: 0x1D,
    0x28: 0x24, 0x29: 0x35, 0x2A: 0x33, 0x2B: 0x30, 0x2C: 0x31, 0x2D: 0x1B, 0x2E: 0x18, 0x2F: 0x21,
    0x30: 0x1E, 0x31: 0x2A, 0x33: 0x29, 0x34: 0x27, 0x35: 0x32, 0x36: 0x2B, 0x37: 0x2F, 0x38: 0x2C,
    0x39: 0x39,
    0x3A: 0x7A, 0x3B: 0x78, 0x3C: 0x63, 0x3D: 0x76, 0x3E: 0x60, 0x3F: 0x61, 0x40: 0x62, 0x41: 0x64,
    0x42: 0x65, 0x43: 0x6D, 0x44: 0x67, 0x45: 0x6F,
    0x4A: 0x73, 0x4B: 0x74, 0x4C: 0x75, 0x4D: 0x77, 0x4E: 0x79, 0x4F: 0x7C, 0x50: 0x7B, 0x51: 0x7D,
    0x52: 0x7E, 0x64: 0x0A,
]

@_cdecl("__previews_injection_jit_link_entrypoint")
public func entry(_ argc: Int32, _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
    MainActor.assumeIsolated {
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        _ = Screen.window
        Server.start()
        app.run()
    }
    return 0
}
