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

@MainActor enum Screen {
    static let window: NSWindow = {
        let w = NSWindow(
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

    static func present(_ c: Captured) -> [String: Any] {
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
}

@_cdecl("__previews_injection_jit_link_entrypoint")
public func entry(_ argc: Int32, _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
    MainActor.assumeIsolated {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        _ = Screen.window
        Server.start()
        app.run()
    }
    return 0
}
