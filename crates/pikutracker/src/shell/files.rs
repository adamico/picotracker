use directories::ProjectDirs;
use pikutracker_core::app::{DirectoryEntry, FileRequest, FileResponse, SettingsContents};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn carry_out(request: FileRequest) -> FileResponse {
    match request {
        FileRequest::List { directory } => match list(&directory) {
            Ok(entries) => FileResponse::Listed { directory, entries },
            Err(error) => failed(directory, &error),
        },
        FileRequest::Read { path } => match fs::read_to_string(&path) {
            Ok(contents) => FileResponse::Read { path, contents },
            Err(error) => failed(path, &error),
        },
        FileRequest::ReadBytes { path } => match fs::read(&path) {
            Ok(contents) => FileResponse::BytesRead { path, contents },
            Err(error) => failed(path, &error),
        },
        FileRequest::Write { path, contents } => written(path, contents.as_bytes()),
        FileRequest::WriteBytes { path, contents } => written(path, &contents),
        FileRequest::Rename { from, to } => match fs::rename(&from, &to) {
            Ok(()) => FileResponse::Renamed { path: to },
            Err(error) => failed(from, &error),
        },
    }
}

pub fn read_settings(path: &Path) -> SettingsContents {
    match fs::read(path) {
        Ok(bytes) => SettingsContents::Bytes(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => SettingsContents::Missing,
        Err(error) => SettingsContents::Unreadable {
            reason: error.to_string(),
        },
    }
}

fn app_folders() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
}

pub fn config_file() -> Option<PathBuf> {
    let folders = app_folders()?;
    Some(folders.config_dir().join("config.ron"))
}

pub fn state_file() -> Option<PathBuf> {
    let folders = app_folders()?;
    let folder = folders.state_dir().unwrap_or(folders.data_local_dir());
    Some(folder.join("state.ron"))
}

fn written(path: PathBuf, contents: &[u8]) -> FileResponse {
    match replace_atomically(&path, contents) {
        Ok(()) => FileResponse::Written { path },
        Err(error) => failed(path, &error),
    }
}

fn failed(path: PathBuf, error: &io::Error) -> FileResponse {
    if error.kind() == io::ErrorKind::NotFound {
        return FileResponse::Missing { path };
    }
    FileResponse::Failed {
        path,
        reason: error.to_string(),
    }
}

fn list(directory: &Path) -> io::Result<Vec<DirectoryEntry>> {
    fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            Ok(DirectoryEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_directory: entry.path().is_dir(),
            })
        })
        .collect()
}

fn replace_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(folder) = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        fs::create_dir_all(folder)?;
    }
    let mut temporary: OsString = path.as_os_str().to_owned();
    temporary.push(".tmp");
    fs::write(&temporary, contents)?;
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}
