use std::collections::HashSet;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::state::AgentState;
use crate::types::{ShortcutDto, SteamUserDto};

#[tauri::command]
pub async fn launch_game(
    app_id: u32,
    state: State<'_, Arc<AgentState>>,
) -> Result<(), String> {
    let cef = capydeploy_steam::CefClient::new();

    // 1. Inject wrapper into launch options (Linux only).
    #[cfg(target_os = "linux")]
    {
        let script_path = state.game_log_wrapper.ensure_script_installed().await?;
        let marker = script_path.display().to_string();
        let escaped_marker = serde_json::to_string(&marker).unwrap_or_default();

        // Inject wrapper + register one-shot lifecycle listener that strips it
        // when the game exits — all in a single CEF evaluate to avoid races.
        let inject_js = format!(
            r#"(function() {{
                const MARKER = {escaped_marker};
                const prefix = MARKER + " {app_id} ";
                const d = appDetailsStore?.GetAppDetails({app_id});
                const cur = d?.strLaunchOptions ?? "";

                if (!cur.includes(MARKER)) {{
                    let opts;
                    if (!cur) {{
                        opts = prefix + "%command%";
                    }} else if (cur.includes("%command%")) {{
                        opts = cur.replace("%command%", prefix + "%command%");
                    }} else {{
                        opts = prefix + cur + " %command%";
                    }}
                    SteamClient.Apps.SetAppLaunchOptions({app_id}, opts);
                }}

                // One-shot lifecycle listener: strip wrapper when game stops.
                const reg = SteamClient.GameSessions.RegisterForAppLifetimeNotifications(
                    (n) => {{
                        if (n.unAppID === {app_id} && !n.bRunning) {{
                            reg.unregister();
                            try {{
                                const d2 = appDetailsStore?.GetAppDetails({app_id});
                                let o = d2?.strLaunchOptions ?? "";
                                if (o.includes(MARKER)) {{
                                    o = o.replace(
                                        new RegExp("\\\\S*" + MARKER.replace(/[.*+?^${{}}()|[\\]\\\\]/g, "\\\\$&") + "\\\\s+" + {app_id} + "\\\\s*", "g"),
                                        ""
                                    ).trim();
                                    if (o === "%command%") o = "";
                                    SteamClient.Apps.SetAppLaunchOptions({app_id}, o);
                                }}
                            }} catch(e) {{}}
                        }}
                    }}
                );
            }})()"#
        );

        cef.evaluate_void(&inject_js)
            .await
            .map_err(|e| format!("failed to inject wrapper: {e}"))?;

        // 2. Start log watcher.
        let log_dir = capydeploy_game_log::log_dir();
        state.game_log_tailer.start_watch(app_id, log_dir).await;
    }

    // 3. Small delay so Steam registers new launch options, then launch.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let launch_js = format!(
        r#"(function() {{
            let gid = String({app_id});
            try {{
                const ov = appStore?.GetAppOverviewByAppID({app_id});
                if (ov?.gameid) gid = String(ov.gameid);
            }} catch(e) {{}}
            SteamClient.Apps.RunGame(gid, "", -1, 100);
        }})()"#
    );

    cef.evaluate_void(&launch_js)
        .await
        .map_err(|e| format!("failed to launch game: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn get_steam_users() -> Result<Vec<SteamUserDto>, String> {
    let users = capydeploy_steam::get_users().map_err(|e| e.to_string())?;
    Ok(users
        .into_iter()
        .map(|u| SteamUserDto {
            id: u.id.clone(),
            name: u.id,
        })
        .collect())
}

#[tauri::command]
pub async fn get_shortcuts(
    user_id: String,
    state: State<'_, Arc<AgentState>>,
) -> Result<Vec<ShortcutDto>, String> {
    let sm = capydeploy_steam::ShortcutManager::new().map_err(|e| e.to_string())?;
    let vdf_path = sm.shortcuts_path(&user_id);

    // Read VDF shortcuts, filtering out app IDs that were deleted via CEF
    // but not yet flushed from the VDF file.
    let deleted = state.deleted_app_ids.lock().await;
    let vdf_list =
        capydeploy_steam::load_shortcuts_vdf(std::path::Path::new(&vdf_path)).unwrap_or_default();

    let mut seen_ids: HashSet<u32> = HashSet::new();
    let mut result: Vec<ShortcutDto> = vdf_list
        .into_iter()
        .filter(|s| !deleted.contains(&s.app_id))
        .map(|s| {
            seen_ids.insert(s.app_id);
            ShortcutDto {
                app_id: s.app_id,
                name: s.name,
                exe: s.exe,
                start_dir: s.start_dir,
            }
        })
        .collect();
    drop(deleted);

    // Merge tracked shortcuts not yet in VDF (CEF-created, not flushed yet).
    let tracked = state.tracked_shortcuts.lock().await;
    for ts in tracked.iter() {
        if !seen_ids.contains(&ts.app_id) {
            result.push(ShortcutDto {
                app_id: ts.app_id,
                name: ts.name.clone(),
                exe: ts.exe.clone(),
                start_dir: ts.start_dir.clone(),
            });
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn delete_shortcut(
    user_id: String,
    app_id: u32,
    state: State<'_, Arc<AgentState>>,
    app: AppHandle,
) -> Result<(), String> {
    // Ensure CEF debug file exists.
    let ctrl = capydeploy_steam::Controller::new();
    let _ = ctrl.ensure_cef_debug_file();

    // Remove via CEF API.
    let cef = capydeploy_steam::CefClient::new();
    cef.remove_shortcut(app_id)
        .await
        .map_err(|e| format!("failed to remove shortcut via CEF: {e}"))?;

    // Remove from tracked list and mark as deleted.
    {
        let mut tracked = state.tracked_shortcuts.lock().await;
        tracked.retain(|s| s.app_id != app_id);
    }
    {
        let mut deleted = state.deleted_app_ids.lock().await;
        deleted.insert(app_id);
    }

    // Delete artwork files (best-effort).
    if let Ok(sm) = capydeploy_steam::ShortcutManager::new() {
        let _ = sm.delete_artwork(&user_id, app_id);
    }

    let _ = app.emit("shortcuts:changed", &());

    Ok(())
}
