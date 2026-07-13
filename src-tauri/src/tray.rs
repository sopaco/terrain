use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

pub const MAIN_WINDOW_ID: &str = "main";
pub const USAGE_WINDOW_ID: &str = "usage";

const MENU_SHOW_MAIN: &str = "show_main";
const MENU_USAGE: &str = "usage_dashboard";
const MENU_ABOUT: &str = "about";
const MENU_QUIT: &str = "quit";

pub fn init(app: &tauri::App) -> tauri::Result<()> {
    attach_main_window_close_handler(app)?;

    let show_main =
        MenuItem::with_id(app, MENU_SHOW_MAIN, "显示主窗口", true, None::<&str>)?;
    let usage =
        MenuItem::with_id(app, MENU_USAGE, "Token 用量看板", true, None::<&str>)?;
    let about = MenuItem::with_id(app, MENU_ABOUT, "关于", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&show_main, &usage, &sep, &about, &sep, &quit],
    )?;

    let icon = tauri::include_image!("icons/64x64.png");

    let _tray = TrayIconBuilder::with_id("terrain-tray")
        .icon(icon)
        .menu(&menu)
        .tooltip("Terrain")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW_MAIN => show_main_window(app),
            MENU_USAGE => open_usage_window(app),
            MENU_ABOUT => show_about(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

pub fn handle_run_event(app: &AppHandle, event: &RunEvent) {
    if let RunEvent::Reopen { .. } = event {
        show_main_window(app);
    }
}

fn attach_main_window_close_handler(app: &tauri::App) -> tauri::Result<()> {
    let Some(main_win) = app.get_webview_window(MAIN_WINDOW_ID) else {
        return Ok(());
    };

    let main_win_clone = main_win.clone();
    main_win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = main_win_clone.hide();
        }
    });

    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_ID) else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

pub fn open_usage_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(USAGE_WINDOW_ID) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }

    let _ = WebviewWindowBuilder::new(app, USAGE_WINDOW_ID, WebviewUrl::App("index.html".into()))
        .title("Token 用量看板")
        .inner_size(720.0, 840.0)
        .min_inner_size(480.0, 560.0)
        .resizable(true)
        .center()
        .build();
}

fn show_about(app: &AppHandle) {
    let version = &app.package_info().version;
    app.dialog()
        .message(format!(
            "Terrain v{version}"
        ))
        .title("About Terrain")
        .kind(MessageDialogKind::Info)
        .show(|_| {});
}
