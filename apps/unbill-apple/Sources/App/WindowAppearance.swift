import SwiftUI
import UIKit

// sirno:witness:unbill-apple:begin
/// Own the appearance at the window boundary so presented sheets update too.
struct WindowAppearance: UIViewRepresentable {
    let appearance: String

    func makeUIView(context: Context) -> WindowAppearanceView {
        WindowAppearanceView()
    }

    func updateUIView(_ view: WindowAppearanceView, context: Context) {
        switch appearance {
        case "light": view.style = .light
        case "dark": view.style = .dark
        default: view.style = .unspecified
        }
    }
}

final class WindowAppearanceView: UIView {
    var style: UIUserInterfaceStyle = .unspecified {
        didSet { applyAppearance() }
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        applyAppearance()
    }

    private func applyAppearance() {
        guard let window, window.overrideUserInterfaceStyle != style else { return }
        window.overrideUserInterfaceStyle = style
    }
}
// sirno:witness:unbill-apple:end
