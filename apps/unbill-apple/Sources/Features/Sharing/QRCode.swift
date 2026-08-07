import CoreImage.CIFilterBuiltins
import UIKit

// Generate a QR code image from a string (e.g. an invitation URL).
enum QRCode {
    private static let context = CIContext()

    static func image(from string: String) -> UIImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(string.utf8)
        filter.correctionLevel = "M"
        guard let output = filter.outputImage else { return nil }
        // The generator output is tiny; scale up with no interpolation.
        let scaled = output.transformed(by: CGAffineTransform(scaleX: 12, y: 12))
        guard let cg = context.createCGImage(scaled, from: scaled.extent) else { return nil }
        return UIImage(cgImage: cg)
    }
}
