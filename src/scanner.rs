use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub fn find_json_files<P: AsRef<Path>>(root: P) -> impl Iterator<Item = PathBuf> {
    WalkBuilder::new(root)
        .follow_links(false)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .map(|e| e.path().to_owned())
}
