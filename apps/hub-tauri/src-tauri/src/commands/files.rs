//! File dialog Tauri commands.

use crate::types::ArtworkFileResultDto;

#[tauri::command]
pub async fn select_folder(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .blocking_pick_folder();

    match path {
        Some(p) => Ok(p.to_string()),
        None => Ok(String::new()),
    }
}

#[tauri::command]
pub async fn select_artwork_file(app: tauri::AppHandle) -> Result<ArtworkFileResultDto, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "ico"])
        .blocking_pick_file();

    match path {
        Some(p) => {
            let path_str = p.to_string();
            let data = std::fs::read(&path_str).map_err(|e| e.to_string())?;
            let size = data.len() as u64;

            // Detect content type from extension.
            let content_type = if path_str.ends_with(".png") {
                "image/png"
            } else if path_str.ends_with(".webp") {
                "image/webp"
            } else if path_str.ends_with(".ico") {
                "image/vnd.microsoft.icon"
            } else {
                "image/jpeg"
            };

            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            let data_uri = format!("data:{content_type};base64,{b64}");

            Ok(ArtworkFileResultDto {
                path: path_str,
                data_uri,
                content_type: content_type.into(),
                size,
            })
        }
        None => Err("no file selected".into()),
    }
}
