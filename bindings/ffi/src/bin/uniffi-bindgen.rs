// Binding generator entry point. Invoked as:
//   cargo run -p unbill-ffi --bin uniffi-bindgen -- \
//     generate --library <compiled lib> --language swift --out-dir <dir>
fn main() {
    uniffi::uniffi_bindgen_main()
}
