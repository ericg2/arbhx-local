use crate::util::SimpleIgnore;
use arbhx_core::{FilterOptions, MetaStream, Metadata, SizedQuery};
use async_trait::async_trait;
use async_walkdir::{DirEntry, Filtering, WalkDir};
use futures_lite::StreamExt;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

pub struct LocalQuery {
    pub(crate) abs: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) opts: FilterOptions,
    pub(crate) sort: SimpleIgnore,
    pub(crate) recursive: bool,
    pub(crate) root: bool,
}

impl LocalQuery {
    pub(crate) fn new(
        abs: &Path,
        path: &Path,
        opts: Option<FilterOptions>,
        recursive: bool,
        root: bool,
    ) -> io::Result<Self> {
        let opts = opts.unwrap_or_default();
        let sort = SimpleIgnore::new(&opts)?;
        Ok(Self {
            sort,
            abs: abs.to_path_buf(),
            path: path.to_path_buf(),
            opts,
            recursive,
            root,
        })
    }

    async fn sort_entry(self: Arc<Self>, entry: DirEntry) -> Filtering {
        // Attempt to check all the rules to see what we need!
        let Some(meta) = entry.metadata().await.ok() else {
            return Filtering::Ignore;
        };
        let ext = Metadata::default()
            .set_path(entry.path())
            .set_is_dir(meta.is_dir())
            .set_mtime(meta.modified().ok().map(|x| x.into()))
            .set_atime(meta.accessed().ok().map(|x| x.into()))
            .set_ctime(meta.created().ok().map(|x| x.into()))
            .set_size(meta.len());
        if !self.root && entry.path() == self.path {
            return Filtering::Ignore; // *** just ignore the single element!
        }
        if !self.recursive {
            if let Ok(stripped) = entry.path().strip_prefix(&self.path) {
                if stripped.components().count() > 1 {
                    return if meta.is_dir() {
                        Filtering::IgnoreDir
                    } else {
                        Filtering::Ignore
                    };
                }
            }
        }
        if let Ok(true) = self.sort.filter_ok(&ext) {
            Filtering::Continue
        } else {
            Filtering::Ignore
        }
    }

    async fn map_entry(
        abs: PathBuf,
        entry: async_walkdir::Result<DirEntry>,
    ) -> io::Result<Metadata> {
        let entry = entry?;
        let meta = entry.metadata().await?;
        let path = crate::fix_path(entry.path(), abs.as_path());
        Ok(Metadata::default()
            .set_path(path)
            .set_is_dir(meta.is_dir())
            .set_mtime(meta.modified().ok().map(|x| x.into()))
            .set_atime(meta.accessed().ok().map(|x| x.into()))
            .set_ctime(meta.created().ok().map(|x| x.into()))
            .set_size(meta.len()))
    }

    pub(crate) async fn build(self: Arc<Self>) -> Pin<Box<MetaStream>> {
        let this = self.clone();
        let abs = self.abs.clone();
        let ret = WalkDir::new(&self.path)
            .filter(move |x| Self::sort_entry(this.clone(), x))
            .then(move |x| {
                let abs = abs.clone();
                Self::map_entry(abs, x)
            });

        Box::pin(ret)
    }
}

#[async_trait]
impl SizedQuery for LocalQuery {
    async fn size(self: Arc<Self>) -> io::Result<Option<u64>> {
        let mut size = 0;
        let mut walk = self.build().await;
        while let Some(item) = walk.next().await {
            match item {
                Ok(meta) => {
                    if meta.is_dir() {
                        size += 0
                    } else {
                        size += meta.size();
                    }
                }
                Err(e) => eprintln!("error: {}", e),
            }
        }
        Ok(Some(size))
    }

    async fn stream(self: Arc<Self>) -> io::Result<Pin<Box<MetaStream>>> {
        Ok(self.build().await)
    }
}
