use qrcode::{QrCode, types::QrResult};

// sirno:witness:unbill-console:begin
/// Render `data` as a compact text QR code using Unicode half-block characters.
/// Returns an error if the input cannot be encoded into a QR code.
pub fn to_text(data: &str) -> QrResult<String> {
    let code = QrCode::new(data.as_bytes())?;
    Ok(code
        .render::<qrcode::render::unicode::Dense1x2>()
        .quiet_zone(true)
        .build())
}

/// Render `data` as a standalone SVG string for embedding in HTML.
/// Returns an error if the input cannot be encoded into a QR code.
pub fn to_svg(data: &str) -> QrResult<String> {
    use qrcode::render::svg;
    let code = QrCode::new(data.as_bytes())?;
    Ok(code.render::<svg::Color>().quiet_zone(true).build())
}
// sirno:witness:unbill-console:end

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_qr_input_returns_capacity_errors() {
        let data = "x".repeat(10_000);
        assert_eq!(to_text(&data), Err(qrcode::types::QrError::DataTooLong));
        assert_eq!(to_svg(&data), Err(qrcode::types::QrError::DataTooLong));
    }

    #[test]
    fn text_output_is_nonempty() {
        let text = to_text("unbill://join/test").unwrap();
        assert!(!text.is_empty());
        assert!(text.contains('\u{2588}'));
    }

    #[test]
    fn svg_output_is_valid() {
        let svg = to_svg("unbill://join/test").unwrap();
        assert!(svg.starts_with("<?xml"));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
