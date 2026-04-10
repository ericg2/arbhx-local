use std::path::{Component, Path, PathBuf};

mod file;
mod iterator;
mod util;
mod vfs;

pub use vfs::LocalVfs;

pub(crate) fn join_force(base: impl AsRef<Path>, p: impl AsRef<Path>) -> PathBuf {
    let mut out = PathBuf::from(base.as_ref());
    for comp in p.as_ref().components() {
        match comp {
            Component::Prefix(_) => {} // skip drive letters / UNC prefix
            Component::RootDir => {}   // skip leading /
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub(crate) fn fix_path(path: PathBuf, prefix: &Path) -> PathBuf {
    // convert both to consistent forward-slash form
    let mut path_s = path.to_string_lossy().replace('\\', "/");
    let prefix_s = prefix.to_string_lossy().replace('\\', "/");

    // remove Windows drive letter if present (D:/tmp -> /tmp-like logic)
    if let Some(idx) = path_s.find(':') {
        path_s = path_s[idx + 1..].to_string();
    }

    if let Some(idx) = prefix_s.find(':') {
        let prefix_s = &prefix_s[idx + 1..];

        // ensure leading slash consistency
        if !path_s.starts_with('/') {
            path_s.insert(0, '/');
        }

        if path_s.starts_with(prefix_s) {
            path_s = path_s[prefix_s.len()..].to_string();
        }
    }

    // ensure leading slash
    if !path_s.starts_with('/') {
        path_s.insert(0, '/');
    }

    // remove trailing slash (except root)
    if path_s.len() > 1 && path_s.ends_with('/') {
        path_s.pop();
    }

    PathBuf::from(path_s)
}