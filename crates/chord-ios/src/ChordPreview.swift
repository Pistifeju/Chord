// The preview helper: a dylib preview.rs builds with swiftc and inserts into the app on a preview simulator. It takes over the app's entry point, finds the app's `#Preview` registry types and shows the first one full screen.
// Private behavior it relies on, checked on Xcode 27.1:
// - Xcode's debug-dylib stub executable calls `__previews_injection_jit_link_entrypoint(argc, argv, …)` first when any image exports it.
// - `Preview` hides its content, so the dylib interposes the public `Preview.init(_:traits:body:)` of SwiftUI and UIKit and keeps the arguments.
// - `PreviewTrait<T>` is one reference to a Swift array of `any PreviewTraitProtocol`; each element has a `value` (`PreviewLayout` for layouts).
import DeveloperToolsSupport
import Foundation
import MachO
import SwiftUI
import UIKit

enum Body {
    case swiftUI(@MainActor () -> any View)
    case controller(@MainActor () -> UIViewController)
    case view(@MainActor () -> UIView)
}

struct Captured {
    let traits: [PreviewTrait<Preview.ViewTraits>]
    let body: Body
}

@MainActor var captured: Captured?

typealias Traits = [PreviewTrait<Preview.ViewTraits>]
typealias SwiftUIInit = @convention(thin) (String?, Traits, @escaping @MainActor () -> any View) -> Preview
typealias ControllerInit = @convention(thin) (String?, Traits, @escaping @MainActor () -> UIViewController) -> Preview
typealias ViewInit = @convention(thin) (String?, Traits, @escaping @MainActor () -> UIView) -> Preview

// The originals are reached through the interpose tuples: naming them in Swift as well crashes swiftc 6.4.
@_silgen_name("chord_swiftui_init")
@MainActor func swiftUIInit(_ name: String?, traits: Traits, body: @escaping @MainActor () -> any View) -> Preview {
    captured = Captured(traits: traits, body: .swiftUI(body))
    return unsafeBitCast(unsafeBitCast(interposeSwiftUI.1, to: UnsafeRawPointer.self), to: SwiftUIInit.self)(name, traits, body)
}

@_silgen_name("chord_controller_init")
@MainActor func controllerInit(_ name: String?, traits: Traits, body: @escaping @MainActor () -> UIViewController) -> Preview {
    captured = Captured(traits: traits, body: .controller(body))
    return unsafeBitCast(unsafeBitCast(interposeController.1, to: UnsafeRawPointer.self), to: ControllerInit.self)(name, traits, body)
}

@_silgen_name("chord_view_init")
@MainActor func viewInit(_ name: String?, traits: Traits, body: @escaping @MainActor () -> UIView) -> Preview {
    captured = Captured(traits: traits, body: .view(body))
    return unsafeBitCast(unsafeBitCast(interposeView.1, to: UnsafeRawPointer.self), to: ViewInit.self)(name, traits, body)
}

// Linker aliases of the functions above (-Xlinker -alias): a second Swift declaration of the same name crashes swiftc.
@_extern(c, "chord_swiftui_init_alias") func swiftUIInitC()
@_extern(c, "chord_controller_init_alias") func controllerInitC()
@_extern(c, "chord_view_init_alias") func viewInitC()
@_extern(c, "$s21DeveloperToolsSupport7PreviewV7SwiftUIE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdAD0J0_pyScMYcctcfC") func originalSwiftUIInit()
@_extern(c, "$s21DeveloperToolsSupport7PreviewV5UIKitE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdSo16UIViewControllerCyScMYcctcfC") func originalControllerInit()
@_extern(c, "$s21DeveloperToolsSupport7PreviewV5UIKitE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdSo6UIViewCyScMYcctcfC") func originalViewInit()

@section("__DATA,__interpose") @used
let interposeSwiftUI: (@convention(c) () -> Void, @convention(c) () -> Void) = (swiftUIInitC, originalSwiftUIInit)
@section("__DATA,__interpose") @used
let interposeController: (@convention(c) () -> Void, @convention(c) () -> Void) = (controllerInitC, originalControllerInit)
@section("__DATA,__interpose") @used
let interposeView: (@convention(c) () -> Void, @convention(c) () -> Void) = (viewInitC, originalViewInit)

enum Layout: Equatable {
    case device
    case sizeThatFits
    case fixed(CGSize)
}

/// `.sizeThatFitsLayout` and `.fixedLayout(width:height:)`; the preview fills the screen for every other trait.
func layout(of traits: Traits) -> Layout {
    guard MemoryLayout<PreviewTrait<Preview.ViewTraits>>.size == 8 else { return .device }
    var out = Layout.device
    for trait in traits {
        let storage = unsafeBitCast(trait, to: UnsafeRawPointer.self)
        let count = storage.load(fromByteOffset: 16, as: Int.self)
        guard (0..<64).contains(count) else { continue }
        for i in 0..<count {
            // A 40-byte existential whose first 32 bytes have the layout of `Any`.
            let value = storage.load(fromByteOffset: 32 + i * 40, as: Any.self)
            for child in Mirror(reflecting: value).children {
                switch child.value as? PreviewLayout {
                case .sizeThatFits?: out = .sizeThatFits
                case .fixed(let w, let h)?: out = .fixed(CGSize(width: w, height: h))
                default: break
                }
            }
        }
    }
    return out
}

struct Entry {
    let fileID: String
    let line: Int
    let type: any PreviewRegistry.Type
}

/// Every `PreviewRegistry` conformance in one image, from its `__swift5_proto` section.
func scan(_ header: UnsafePointer<mach_header>) -> [Entry] {
    guard let proto = dlsym(UnsafeMutableRawPointer(bitPattern: -2), "$s21DeveloperToolsSupport15PreviewRegistryMp") else { return [] }
    var found: [Entry] = []
    var size: UInt = 0
    guard let section = getsectiondata(UnsafeRawPointer(header).assumingMemoryBound(to: mach_header_64.self), "__TEXT", "__swift5_proto", &size) else { return [] }
    let base = UnsafeRawPointer(section)
    for k in 0..<Int(size) / 4 {
        let entry = base + k * 4
        let descriptor = entry + Int(entry.load(as: Int32.self))
        // A relative pointer to the protocol, indirect when its low bit is set.
        let protoOffset = descriptor.load(as: Int32.self)
        var p = descriptor + Int(protoOffset & ~1)
        if protoOffset & 1 != 0 { p = p.load(as: UnsafeRawPointer.self) }
        guard p == UnsafeRawPointer(proto) else { continue }
        let kind = ((descriptor + 12).load(as: UInt32.self) >> 3) & 7
        let typeField = descriptor + 4
        var type = typeField + Int(typeField.load(as: Int32.self))
        switch kind {
        case 0: break
        case 1: type = type.load(as: UnsafeRawPointer.self)
        default: continue
        }
        // Type context descriptor: flags, parent, name, then the relative access function.
        let accessField = type + 12
        let access = accessField + Int(accessField.load(as: Int32.self))
        let metadata = unsafeBitCast(access, to: (@convention(c) (Int) -> UnsafeRawPointer).self)(0)
        guard let registry = unsafeBitCast(metadata, to: Any.Type.self) as? any PreviewRegistry.Type else { continue }
        found.append(Entry(fileID: registry.fileID, line: registry.line, type: registry))
    }
    return found
}

/// The app bundle's previews in file and line order.
@MainActor func previews() -> [Entry] {
    let path = "/" + (Bundle.main.bundlePath as NSString).lastPathComponent + "/"
    let all = (0..<_dyld_image_count()).flatMap { i -> [Entry] in
        guard let header = _dyld_get_image_header(i), let name = _dyld_get_image_name(i), String(cString: name).contains(path) else { return [] }
        return scan(header)
    }
    return all.sorted { ($0.fileID, $0.line) < ($1.fileID, $1.line) }
}

@MainActor func capture(_ type: any PreviewRegistry.Type) -> Captured? {
    captured = nil
    _ = try? type.makePreview()
    defer { captured = nil }
    return captured
}

final class Centered: UIViewController {
    let size: CGSize?
    let child: UIView
    init(_ child: UIView, size: CGSize?) {
        self.child = child
        self.size = size
        super.init(nibName: nil, bundle: nil)
    }
    required init?(coder: NSCoder) { nil }
    override func viewDidLoad() {
        view.backgroundColor = .systemBackground
        child.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(child)
        var c = [child.centerXAnchor.constraint(equalTo: view.centerXAnchor), child.centerYAnchor.constraint(equalTo: view.centerYAnchor)]
        if let size {
            c += [child.widthAnchor.constraint(equalToConstant: size.width), child.heightAnchor.constraint(equalToConstant: size.height)]
        }
        NSLayoutConstraint.activate(c)
    }
}

final class Pinned: UIViewController {
    let child: UIView
    init(_ child: UIView) {
        self.child = child
        super.init(nibName: nil, bundle: nil)
    }
    required init?(coder: NSCoder) { nil }
    override func loadView() { view = child }
}

@MainActor func controller(for c: Captured) -> UIViewController {
    let layout = layout(of: c.traits)
    let size: CGSize? = if case .fixed(let s) = layout { s } else { nil }
    switch c.body {
    case .controller(let make):
        return make()
    case .view(let make):
        return layout == .device ? Pinned(make()) : Centered(make(), size: size)
    case .swiftUI(let make):
        let host = UIHostingController(rootView: AnyView(make()))
        guard layout != .device else { return host }
        host.sizingOptions = .intrinsicContentSize
        host.view.backgroundColor = .clear
        let box = Centered(host.view, size: size)
        box.addChild(host)
        host.didMove(toParent: box)
        return box
    }
}

@MainActor enum Screen {
    static var window: UIWindow?

    static func attach(_ scene: UIScene) {
        guard window == nil, let scene = scene as? UIWindowScene else { return }
        let w = UIWindow(windowScene: scene)
        w.backgroundColor = .systemBackground
        w.rootViewController = previews().lazy.compactMap { capture($0.type) }.first.map(controller(for:)) ?? UIViewController()
        w.makeKeyAndVisible()
        window = w
    }
}

@objc(ChordPreviewDelegate)
final class Delegate: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        // Restored scene sessions skip `configurationForConnecting`, so the window attaches on the connect notification.
        NotificationCenter.default.addObserver(forName: UIScene.willConnectNotification, object: nil, queue: .main) { n in
            guard let scene = n.object as? UIScene else { return }
            MainActor.assumeIsolated { Screen.attach(scene) }
        }
        return true
    }

    // Without it iOS 27 traps for a missing scene lifecycle, though a restored session never calls it.
    func application(_ application: UIApplication, configurationForConnecting session: UISceneSession, options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let c = UISceneConfiguration(name: nil, sessionRole: session.role)
        c.delegateClass = SceneDelegate.self
        return c
    }
}

@objc(ChordPreviewSceneDelegate)
final class SceneDelegate: UIResponder, UIWindowSceneDelegate {}

@_cdecl("__previews_injection_jit_link_entrypoint")
public func entry(_ argc: Int32, _ argv: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
    // Registers the class under its @objc name before UIApplicationMain looks it up.
    _ = Delegate.self
    return UIApplicationMain(argc, argv, nil, "ChordPreviewDelegate")
}
