mod export;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(export::ExportJobs::default())
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem, Submenu};

            let config_item =
                MenuItem::with_id(app, "config", "Configuration", true, None::<&str>)?;
            let project_open = MenuItem::with_id(app, "project_open", "Open", true, None::<&str>)?;
            let project_new = MenuItem::with_id(app, "project_new", "New", true, None::<&str>)?;
            let help_item = MenuItem::with_id(app, "help", "Help", true, None::<&str>)?;
            let about_item =
                MenuItem::with_id(app, "about", "About Ernest v0.1.0+0037", true, None::<&str>)?;

            let app_menu = Submenu::with_items(app, "Ernest", true, &[&config_item])?;
            let project_menu =
                Submenu::with_items(app, "Project", true, &[&project_open, &project_new])?;
            let help_menu = Submenu::with_items(app, "Help", true, &[&help_item, &about_item])?;
            let menu = Menu::with_items(app, &[&app_menu, &project_menu, &help_menu])?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            use tauri::Emitter;
            use tauri_plugin_dialog::DialogExt;

            match event.id().as_ref() {
                "project_open" => {
                    let _ = app.emit("project:open", ());
                }
                "project_new" => {
                    let _ = app.emit("project:new", ());
                }
                "help" => {
                    let message = "Ernest Help\n\n\
Open folder: Choose a folder to list Markdown files.\n\
Open file: Open a single Markdown file.\n\
Metadata: Fill the frontmatter fields; errors show required items.\n\
Apply / Normalize: Regenerates frontmatter from metadata and keeps the body.\n\
Save: Writes the file to disk.";
                    app.dialog().message(message).title("Help").blocking_show();
                }
                "about" => {
                    app.dialog()
                        .message("Ernest v0.1.0+0037\nMarkdown + frontmatter workspace")
                        .title("About")
                        .blocking_show();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            export::export_file_async,
            export::cancel_export,
            export::cleanup_export,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
