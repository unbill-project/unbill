use qrcode::QrCode;

/// Render `data` as a compact text QR code using Unicode half-block characters.
pub fn to_text(data: &str) -> String {
    let code = QrCode::new(data.as_bytes()).expect("QR encoding failed");
    code.render::<qrcode::render::unicode::Dense1x2>()
        .quiet_zone(true)
        .build()
}

/// Render `data` as a standalone SVG string for embedding in HTML.
pub fn to_svg(data: &str) -> String {
    use qrcode::render::svg;
    let code = QrCode::new(data.as_bytes()).expect("QR encoding failed");
    code.render::<svg::Color>().quiet_zone(true).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_output_is_nonempty() {
        let text = to_text("unbill://join/test");
        assert!(!text.is_empty());
        assert!(text.contains('\u{2588}'));
    }

    #[test]
    fn svg_output_is_valid() {
        let svg = to_svg("unbill://join/test");
        assert!(svg.starts_with("<?xml"));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
