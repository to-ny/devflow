use tauri::{
    menu::{Menu, MenuItem, Submenu},
    App, AppHandle, Emitter, Manager,
};

pub fn setup(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    // On Linux/WSL2, menu creation can fail due to dbus issues
    // We make this non-fatal to allow the app to start without menus
    match create_menu(app) {
        Ok(menu) => {
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.set_menu(menu) {
                    eprintln!("Warning: Failed to set window menu: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to create menu: {}", e);
        }
    }

    Ok(())
}

fn create_menu(app: &App) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    // File menu
    let open_project = MenuItem::with_id(app, "open_project", "Open Project", true, None::<&str>)?;
    let close_project =
        MenuItem::with_id(app, "close_project", "Close Project", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let file_menu =
        Submenu::with_items(app, "File", true, &[&open_project, &close_project, &quit])?;

    // View menu
    let view_chat = MenuItem::with_id(app, "view_chat", "Chat", true, None::<&str>)?;
    let view_changes = MenuItem::with_id(app, "view_changes", "Changes", true, None::<&str>)?;
    let view_settings = MenuItem::with_id(app, "view_settings", "Settings", true, None::<&str>)?;

    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[&view_chat, &view_changes, &view_settings],
    )?;

    let menu = Menu::with_items(app, &[&file_menu, &view_menu])?;

    Ok(menu)
}

pub fn handle_event(app: &AppHandle, event_id: &str) {
    let window = app.get_webview_window("main");

    match event_id {
        "open_project" => {
            if let Some(win) = window {
                let _ = win.emit("menu-open-project", ());
            }
        }
        "close_project" => {
            if let Some(win) = window {
                let _ = win.emit("menu-close-project", ());
            }
        }
        "view_chat" => {
            if let Some(win) = window {
                let _ = win.emit("menu-navigate", "chat");
            }
        }
        "view_changes" => {
            if let Some(win) = window {
                let _ = win.emit("menu-navigate", "changes");
            }
        }
        "view_settings" => {
            if let Some(win) = window {
                let _ = win.emit("menu-navigate", "settings");
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}
