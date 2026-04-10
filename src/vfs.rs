use crate::file::VfsReadWrite;
use crate::fix_path;
use crate::iterator::LocalQuery;
use arbhx_core::{
    DataFull, DataRead, DataReadSeek, DataUsage, DataWrite, DataWriteSeek, FilterOptions, Metadata,
    SizedQuery, VfsBackend, VfsFull, VfsReader, VfsSeekWriter, VfsWriter,
};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use filetime::{FileTime, set_symlink_file_times};
use std::fs::OpenOptions;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sysinfo::Disks;
use uuid::Uuid;

#[derive(Debug)]
pub struct LocalVfs {
    id: Uuid,
    root: PathBuf,
}

impl LocalVfs {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            id: Uuid::new_v4(),
            root: root.as_ref().to_path_buf(),
        }
    }

    fn join_force(&self, p: &Path) -> PathBuf {
        crate::join_force(&self.root, p)
    }

    async fn raw_metadata(&self, path: &Path) -> io::Result<Option<Metadata>> {
        if !tokio::fs::try_exists(&path).await? {
            return Ok(None);
        }
        let meta = tokio::fs::metadata(&path).await?;
        let path = fix_path(path.to_path_buf(), &self.root);
        let x_meta = Metadata::default()
            .set_path(path)
            .set_is_dir(meta.is_dir())
            .set_mtime(meta.modified().ok().map(|x| x.into()))
            .set_ctime(meta.created().ok().map(|x| x.into()))
            .set_size(meta.len());
        Ok(Some(x_meta))
    }
}

#[async_trait]
impl VfsBackend for LocalVfs {
    fn id(&self) -> Uuid {
        self.id
    }

    fn realpath(&self, item: &Path) -> PathBuf {
        self.join_force(item)
    }

    fn reader(self: Arc<Self>) -> Option<Arc<dyn VfsReader>> {
        Some(self.clone())
    }

    fn writer(self: Arc<Self>) -> Option<Arc<dyn VfsWriter>> {
        Some(self.clone())
    }

    fn writer_seek(self: Arc<Self>) -> Option<Arc<dyn VfsSeekWriter>> {
        Some(self.clone())
    }

    fn full(self: Arc<Self>) -> Option<Arc<dyn VfsFull>> {
        Some(self.clone())
    }

    async fn get_usage(&self) -> std::io::Result<Option<DataUsage>> {
        let disks = Disks::new_with_refreshed_list();
        let ret = disks
            .iter()
            .find(|x| self.root.starts_with(x.mount_point()))
            .map(|disk| {
                let max_bytes = disk.total_space(); // total bytes
                let free_bytes = disk.available_space(); // free bytes
                let used_bytes = max_bytes - free_bytes;
                DataUsage {
                    used_bytes,
                    max_bytes,
                    free_bytes,
                }
            })
            .ok_or(ErrorKind::Unsupported)?;
        Ok(Some(ret))
    }
}

#[async_trait]
impl VfsReader for LocalVfs {
    async fn open_read_start(&self, item: &Path) -> std::io::Result<Box<dyn DataRead>> {
        let path = self.join_force(item);
        VfsReadWrite::read_start(&path).await
    }

    async fn open_read_seek(&self, item: &Path) -> std::io::Result<Box<dyn DataReadSeek>> {
        let path = self.join_force(item);
        Ok(VfsReadWrite::read_random(&path).await?)
    }

    async fn get_metadata(&self, item: &Path) -> std::io::Result<Option<Metadata>> {
        let path = self.join_force(item);
        self.raw_metadata(&path).await
    }

    async fn list(
        &self,
        item: &Path,
        opts: Option<FilterOptions>,
        recursive: bool,
        include_root: bool,
    ) -> std::io::Result<Arc<dyn SizedQuery>> {
        let path = self.join_force(item);
        let ret = LocalQuery::new(&self.root, &path, opts, recursive, include_root)?;
        Ok(Arc::new(ret))
    }
}

#[async_trait]
impl VfsWriter for LocalVfs {
    async fn remove_dir(&self, dirname: &Path) -> io::Result<()> {
        let path = self.join_force(dirname);
        tokio::fs::remove_dir_all(&path).await?;
        Ok(())
    }

    async fn remove_file(&self, filename: &Path) -> io::Result<()> {
        let path = self.join_force(filename);
        tokio::fs::remove_file(&path).await?;
        Ok(())
    }

    async fn create_dir(&self, item: &Path) -> io::Result<()> {
        let path = self.join_force(item);
        tokio::fs::create_dir_all(&path).await?;
        Ok(())
    }

    async fn set_times(
        &self,
        item: &Path,
        mtime: DateTime<Local>,
        atime: DateTime<Local>,
    ) -> std::io::Result<()> {
        let path = self.join_force(item);
        set_symlink_file_times(
            path,
            FileTime::from_system_time(atime.into()),
            FileTime::from_system_time(mtime.into()),
        )?;
        Ok(())
    }

    async fn set_length(&self, item: &Path, size: u64) -> std::io::Result<()> {
        let path = self.join_force(item);
        VfsReadWrite::set_length(path, size).await?;
        Ok(())
    }

    async fn move_to(&self, old: &Path, new: &Path) -> std::io::Result<()> {
        let old = self.join_force(old);
        let new = self.join_force(new);
        tokio::fs::rename(old, new).await?;
        Ok(())
    }

    async fn copy_to(&self, old: &Path, new: &Path) -> std::io::Result<()> {
        let old = self.join_force(old);
        let new = self.join_force(new);
        tokio::fs::copy(old, new).await?;
        Ok(())
    }

    async fn open_write(
        &self,
        item: &Path,
        overwrite: bool,
    ) -> std::io::Result<Box<dyn DataWrite>> {
        let path = self.join_force(item);
        VfsReadWrite::write_append(&path, overwrite).await
    }
}

#[async_trait]
impl VfsSeekWriter for LocalVfs {
    async fn open_write_seek(&self, item: &Path) -> std::io::Result<Box<dyn DataWriteSeek>> {
        let path = self.join_force(item);
        Ok(VfsReadWrite::write_random(&path).await?)
    }
}

#[async_trait]
impl VfsFull for LocalVfs {
    async fn open_full_seek(&self, item: &Path) -> std::io::Result<Box<dyn DataFull>> {
        let path = self.join_force(item);
        Ok(VfsReadWrite::full_random(&path).await?)
    }
}
