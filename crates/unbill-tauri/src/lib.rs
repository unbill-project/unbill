// unbill-tauri: thin Tauri backend.
// Wraps UnbillService methods as Tauri commands and forwards ServiceEvents
// to the frontend via app.emit("unbill:event", ...).
// Implementation begins at M5. See DESIGN.md §11.2.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|_app| {
            // M5: initialize UnbillService here
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running unbill");
}
