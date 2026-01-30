use std::path::{Path, PathBuf};

pub fn find_project_root(file_path: &Path) -> Option<PathBuf> {
    let start = if file_path.is_dir() {
        file_path
    } else {
        file_path.parent()?
    };

    for ancestor in start.ancestors() {
        if ancestor.join(".export.toml").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}
