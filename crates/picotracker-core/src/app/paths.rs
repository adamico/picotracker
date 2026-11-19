//! Path maths done without asking the disk.

use std::path::{Component, Path, PathBuf};

/// A path with every `.` and `..` worked out, without asking the disk.
pub(super) fn resolved(path: &Path) -> Option<PathBuf> {
    let mut resolved = PathBuf::new();
    for component in std::path::absolute(path).ok()?.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other),
        }
    }
    Some(resolved)
}

/// The way from a folder to a file. Paths that share no root stay absolute.
pub(super) fn relative_path(folder: &Path, file: &Path) -> Option<PathBuf> {
    let (folder, file) = (resolved(folder)?, resolved(file)?);
    let shared = folder
        .components()
        .zip(file.components())
        .take_while(|(from, to)| from == to)
        .count();
    if shared == 0 {
        return Some(file);
    }
    let climbs = folder
        .components()
        .skip(shared)
        .map(|_| Component::ParentDir);
    Some(climbs.chain(file.components().skip(shared)).collect())
}
