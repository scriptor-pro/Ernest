mod credentials;
mod export;
mod project;
mod publish;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(export::ExportJobs::default())
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem, Submenu};

            let app_about =
                MenuItem::with_id(app, "app_about", "About Ernest", true, None::<&str>)?;
            let app_preferences =
                MenuItem::with_id(app, "app_preferences", "Preferences...", true, None::<&str>)?;
            let app_updates = MenuItem::with_id(
                app,
                "app_updates",
                "Check for Updates...",
                true,
                None::<&str>,
            )?;
            let app_quit = MenuItem::with_id(app, "app_quit", "Quit", true, None::<&str>)?;

            let project_new =
                MenuItem::with_id(app, "project_new", "New Project...", true, None::<&str>)?;
            let project_open =
                MenuItem::with_id(app, "project_open", "Open Folder...", true, None::<&str>)?;
            let project_recent_empty = MenuItem::with_id(
                app,
                "project_recent_empty",
                "No recent projects",
                false,
                None::<&str>,
            )?;
            let project_settings = MenuItem::with_id(
                app,
                "project_settings",
                "Project Settings...",
                true,
                None::<&str>,
            )?;

            let file_new = MenuItem::with_id(app, "file_new", "New File", true, None::<&str>)?;
            let file_open =
                MenuItem::with_id(app, "file_open", "Open File...", true, None::<&str>)?;
            let file_save = MenuItem::with_id(app, "file_save", "Save", true, None::<&str>)?;
            let file_save_as =
                MenuItem::with_id(app, "file_save_as", "Save As...", true, None::<&str>)?;
            let file_close =
                MenuItem::with_id(app, "file_close", "Close File", true, None::<&str>)?;

            let edit_undo = MenuItem::with_id(app, "edit_undo", "Undo", true, None::<&str>)?;
            let edit_redo = MenuItem::with_id(app, "edit_redo", "Redo", true, None::<&str>)?;
            let edit_cut = MenuItem::with_id(app, "edit_cut", "Cut", true, None::<&str>)?;
            let edit_copy = MenuItem::with_id(app, "edit_copy", "Copy", true, None::<&str>)?;
            let edit_paste = MenuItem::with_id(app, "edit_paste", "Paste", true, None::<&str>)?;
            let edit_select_all =
                MenuItem::with_id(app, "edit_select_all", "Select All", true, None::<&str>)?;

            let doc_apply = MenuItem::with_id(
                app,
                "doc_apply",
                "Apply / Normalize Frontmatter",
                true,
                None::<&str>,
            )?;
            let doc_merge_replace = MenuItem::with_id(
                app,
                "doc_merge_replace",
                "Merge / Replace Frontmatter...",
                true,
                None::<&str>,
            )?;

            let view_toggle_explorer = MenuItem::with_id(
                app,
                "view_toggle_explorer",
                "Toggle File Explorer",
                true,
                None::<&str>,
            )?;
            let view_toggle_metadata = MenuItem::with_id(
                app,
                "view_toggle_metadata",
                "Toggle Metadata Panel",
                true,
                None::<&str>,
            )?;
            let view_toggle_toolbar = MenuItem::with_id(
                app,
                "view_toggle_toolbar",
                "Toggle Toolbar",
                true,
                None::<&str>,
            )?;

            let help_item = MenuItem::with_id(app, "help", "Help", true, None::<&str>)?;
            let help_shortcuts = MenuItem::with_id(
                app,
                "help_shortcuts",
                "Keyboard Shortcuts",
                true,
                None::<&str>,
            )?;
            let help_report =
                MenuItem::with_id(app, "help_report", "Report Issue", true, None::<&str>)?;
            let help_logs = MenuItem::with_id(app, "help_logs", "View Logs", true, None::<&str>)?;

            let app_menu = Submenu::with_items(
                app,
                "Application",
                true,
                &[&app_about, &app_preferences, &app_updates, &app_quit],
            )?;
            let recent_menu =
                Submenu::with_items(app, "Recent Projects", true, &[&project_recent_empty])?;
            let project_menu = Submenu::with_items(
                app,
                "Project",
                true,
                &[&project_new, &project_open, &recent_menu, &project_settings],
            )?;
            let file_menu = Submenu::with_items(
                app,
                "File",
                true,
                &[
                    &file_new,
                    &file_open,
                    &file_save,
                    &file_save_as,
                    &file_close,
                ],
            )?;
            let edit_menu = Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &edit_undo,
                    &edit_redo,
                    &edit_cut,
                    &edit_copy,
                    &edit_paste,
                    &edit_select_all,
                ],
            )?;
            let document_menu =
                Submenu::with_items(app, "Document", true, &[&doc_apply, &doc_merge_replace])?;
            let view_menu = Submenu::with_items(
                app,
                "View",
                true,
                &[
                    &view_toggle_explorer,
                    &view_toggle_metadata,
                    &view_toggle_toolbar,
                ],
            )?;
            let help_menu = Submenu::with_items(
                app,
                "Help",
                true,
                &[&help_item, &help_shortcuts, &help_report, &help_logs],
            )?;
            let menu = Menu::with_items(
                app,
                &[
                    &app_menu,
                    &project_menu,
                    &file_menu,
                    &edit_menu,
                    &document_menu,
                    &view_menu,
                    &help_menu,
                ],
            )?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            use tauri::Emitter;
            use tauri_plugin_dialog::DialogExt;

            match event.id().as_ref() {
                "app_about" => {
                    app.dialog()
                        .message("Ernest v0.2.2+0005\nMarkdown + frontmatter workspace")
                        .title("About")
                        .blocking_show();
                }
                "app_preferences" => {
                    let _ = app.emit("app:preferences", ());
                }
                "app_updates" => {
                    let _ = app.emit("app:updates", ());
                }
                "app_quit" => {
                    app.exit(0);
                }
                "project_open" => {
                    let _ = app.emit("project:open", ());
                }
                "project_new" => {
                    let _ = app.emit("project:new", ());
                }
                "project_settings" => {
                    let _ = app.emit("project:settings", ());
                }
                "file_new" => {
                    let _ = app.emit("file:new", ());
                }
                "file_open" => {
                    let _ = app.emit("file:open", ());
                }
                "file_save" => {
                    let _ = app.emit("file:save", ());
                }
                "file_save_as" => {
                    let _ = app.emit("file:save_as", ());
                }
                "file_close" => {
                    let _ = app.emit("file:close", ());
                }
                "edit_undo" => {
                    let _ = app.emit("edit:undo", ());
                }
                "edit_redo" => {
                    let _ = app.emit("edit:redo", ());
                }
                "edit_cut" => {
                    let _ = app.emit("edit:cut", ());
                }
                "edit_copy" => {
                    let _ = app.emit("edit:copy", ());
                }
                "edit_paste" => {
                    let _ = app.emit("edit:paste", ());
                }
                "edit_select_all" => {
                    let _ = app.emit("edit:select_all", ());
                }
                "doc_apply" => {
                    let _ = app.emit("document:apply", ());
                }
                "doc_merge_replace" => {
                    let _ = app.emit("document:merge_replace", ());
                }
                "view_toggle_explorer" => {
                    let _ = app.emit("view:toggle_explorer", ());
                }
                "view_toggle_metadata" => {
                    let _ = app.emit("view:toggle_metadata", ());
                }
                "view_toggle_toolbar" => {
                    let _ = app.emit("view:toggle_toolbar", ());
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
                "help_shortcuts" => {
                    let _ = app.emit("help:shortcuts", ());
                }
                "help_report" => {
                    let _ = app.emit("help:report", ());
                }
                "help_logs" => {
                    let _ = app.emit("help:logs", ());
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            export::export_file_async,
            export::cancel_export,
            export::cleanup_export,
            credentials::get_credential,
            credentials::set_credential,
            credentials::delete_credential,
            publish::publish_project,
            publish::deploy_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
