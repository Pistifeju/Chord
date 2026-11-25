import DeveloperToolsSupport
import SwiftUI
import UIKit

enum Body {
    case swiftUI(@MainActor () -> any View)
    case controller(@MainActor () -> UIViewController)
    case view(@MainActor () -> UIView)
}

typealias ControllerInit = @convention(thin) (String?, Traits, @escaping @MainActor () -> UIViewController) -> Preview
typealias ViewInit = @convention(thin) (String?, Traits, @escaping @MainActor () -> UIView) -> Preview

@_silgen_name("chord_controller_init")
@MainActor func controllerInit(_ name: String?, traits: Traits, body: @escaping @MainActor () -> UIViewController) -> Preview {
    captured = Captured(name: name, traits: traits, body: .controller(body))
    return unsafeBitCast(unsafeBitCast(interposeController.1, to: UnsafeRawPointer.self), to: ControllerInit.self)(name, traits, body)
}

@_silgen_name("chord_view_init")
@MainActor func viewInit(_ name: String?, traits: Traits, body: @escaping @MainActor () -> UIView) -> Preview {
    captured = Captured(name: name, traits: traits, body: .view(body))
    return unsafeBitCast(unsafeBitCast(interposeView.1, to: UnsafeRawPointer.self), to: ViewInit.self)(name, traits, body)
}

@_extern(c, "chord_controller_init_alias") func controllerInitC()
@_extern(c, "chord_view_init_alias") func viewInitC()
@_extern(c, "$s21DeveloperToolsSupport7PreviewV5UIKitE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdSo16UIViewControllerCyScMYcctcfC") func originalControllerInit()
@_extern(c, "$s21DeveloperToolsSupport7PreviewV5UIKitE_6traits4bodyACSSSg_AA0D5TraitVyAC10ViewTraitsOGdSo6UIViewCyScMYcctcfC") func originalViewInit()

@section("__DATA,__interpose") @used
let interposeController: (@convention(c) () -> Void, @convention(c) () -> Void) = (controllerInitC, originalControllerInit)
@section("__DATA,__interpose") @used
let interposeView: (@convention(c) () -> Void, @convention(c) () -> Void) = (viewInitC, originalViewInit)

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
        w.rootViewController = UIViewController()
        w.makeKeyAndVisible()
        window = w
        Server.start()
    }

    static func contextID() -> UInt32? { nil }

    static func present(_ c: Captured) -> [String: Any] {
        window?.rootViewController = controller(for: c)
        return ["ok": true]
    }
}

@objc(ChordPreviewDelegate)
final class Delegate: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        NotificationCenter.default.addObserver(forName: UIScene.willConnectNotification, object: nil, queue: .main) { n in
            guard let scene = n.object as? UIScene else { return }
            MainActor.assumeIsolated { Screen.attach(scene) }
        }
        return true
    }

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
    _ = Delegate.self
    return UIApplicationMain(argc, argv, nil, "ChordPreviewDelegate")
}
