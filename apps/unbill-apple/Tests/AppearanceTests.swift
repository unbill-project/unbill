import UIKit
import XCTest
@testable import unbill

@MainActor
final class AppearanceTests: XCTestCase {
    func testAppearanceSelectedBeforeWindowAttachmentIsApplied() throws {
        let bridge = WindowAppearanceView()
        bridge.style = .dark
        let window = makeWindow()
        defer { window.isHidden = true }
        window.rootViewController!.view.addSubview(bridge)
        XCTAssertEqual(window.overrideUserInterfaceStyle, .dark)
        bridge.style = .light
        XCTAssertEqual(window.overrideUserInterfaceStyle, .light)
        bridge.style = .unspecified
        XCTAssertEqual(window.overrideUserInterfaceStyle, .unspecified)
    }

    func testDarkToLightUpdatesAnAlreadyPresentedSettingsWindow() async throws {
        let window = makeWindow()
        defer { window.isHidden = true }
        let root = window.rootViewController!
        let bridge = WindowAppearanceView()
        root.view.addSubview(bridge)
        bridge.style = .dark
        window.updateTraitsIfNeeded()
        let settings = UIViewController()
        await withCheckedContinuation { continuation in
            root.present(settings, animated: false) { continuation.resume() }
        }
        XCTAssertEqual(settings.traitCollection.userInterfaceStyle, .dark)
        bridge.style = .light
        window.updateTraitsIfNeeded()
        XCTAssertEqual(window.overrideUserInterfaceStyle, .light)
        XCTAssertEqual(root.traitCollection.userInterfaceStyle, .light)
        XCTAssertEqual(settings.traitCollection.userInterfaceStyle, .light)
        bridge.style = .dark
        window.updateTraitsIfNeeded()
        XCTAssertEqual(settings.traitCollection.userInterfaceStyle, .dark)
        bridge.style = .unspecified
        XCTAssertEqual(window.overrideUserInterfaceStyle, .unspecified)
        root.dismiss(animated: false)
    }

    private func makeWindow() -> UIWindow {
        let scene = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first!
        let window = UIWindow(windowScene: scene)
        window.rootViewController = UIViewController()
        window.makeKeyAndVisible()
        return window
    }
}
