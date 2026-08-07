import SwiftUI
import VisionKit

// Live QR scanner (VisionKit). Device-only — DataScannerViewController is not
// available in the Simulator or on unsupported hardware; callers should gate on
// `QRScannerView.isAvailable`.
struct QRScannerView: UIViewControllerRepresentable {
    var onScan: (String) -> Void

    static var isAvailable: Bool {
        DataScannerViewController.isSupported && DataScannerViewController.isAvailable
    }

    func makeUIViewController(context: Context) -> DataScannerViewController {
        let scanner = DataScannerViewController(
            recognizedDataTypes: [.barcode(symbologies: [.qr])],
            qualityLevel: .balanced,
            isHighFrameRateTrackingEnabled: false,
            isHighlightingEnabled: true
        )
        scanner.delegate = context.coordinator
        return scanner
    }

    func updateUIViewController(_ vc: DataScannerViewController, context: Context) {
        try? vc.startScanning()
    }

    func makeCoordinator() -> Coordinator { Coordinator(onScan: onScan) }

    final class Coordinator: NSObject, DataScannerViewControllerDelegate {
        let onScan: (String) -> Void
        private var handled = false
        init(onScan: @escaping (String) -> Void) { self.onScan = onScan }

        func dataScanner(
            _ scanner: DataScannerViewController,
            didAdd addedItems: [RecognizedItem],
            allItems: [RecognizedItem]
        ) {
            guard !handled else { return }
            for item in addedItems {
                if case let .barcode(barcode) = item, let text = barcode.payloadStringValue {
                    handled = true
                    onScan(text)
                    break
                }
            }
        }
    }
}
