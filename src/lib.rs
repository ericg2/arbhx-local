use std::path::{Component, Path, PathBuf};

mod file;
mod iterator;
mod util;
mod vfs;

pub use vfs::LocalVfs;

#[cfg(test)]
mod tests {
    use crate::LocalVfs;
    use arbhx_core::{VfsBackend, VfsReader, VfsWriter};
    use futures_lite::StreamExt;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[tokio::test]
    async fn do_test() {
        let base_dir = PathBuf::from("D:\\tmp");
        tokio::fs::create_dir_all(&base_dir).await.unwrap();

        let vfs = Arc::new(LocalVfs::new("test", base_dir.clone()));
        let mut st = vfs
            .read_dir("/".as_ref(), None, true, true)
            .await
            .unwrap()
            .stream()
            .await
            .unwrap();
        while let Some(item) = st.next().await {
            let item = item.unwrap();
            println!("{:?}", item);
        }

        vfs.create_dir("/test".as_ref()).await.unwrap();
        vfs.open_read_random("/rider.exe".as_ref()).await;
    }
}

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