use arbhx_core::{DataFull, DataRead, DataReadSeek, DataWrite, DataWriteSeek};
use async_trait::async_trait;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::SeekFrom;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

#[derive(Debug)]
pub struct VfsReadWrite {
    file: tokio::fs::File,
}

impl VfsReadWrite {
    pub async fn read_start(path: impl AsRef<Path>) -> io::Result<Box<dyn DataRead>> {
        let file = OpenOptions::new().read(true).open(path).await?;
        let ret = VfsReadWrite { file };
        Ok(Box::new(ret))
    }
    pub async fn read_random(path: impl AsRef<Path>) -> io::Result<Box<dyn DataReadSeek>> {
        let file = OpenOptions::new().read(true).open(path).await?;
        let ret = VfsReadWrite { file };
        Ok(Box::new(ret))
    }
    pub async fn write_append(
        path: impl AsRef<Path>,
        overwrite: bool,
    ) -> io::Result<Box<dyn DataWrite>> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .truncate(overwrite)
            .open(path)
            .await?;
        let ret = VfsReadWrite { file };
        Ok(Box::new(ret))
    }
    pub async fn write_random(path: impl AsRef<Path>) -> io::Result<Box<dyn DataWriteSeek>> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .await?;
        let ret = VfsReadWrite { file };
        Ok(Box::new(ret))
    }
    pub async fn full_random(path: impl AsRef<Path>) -> io::Result<Box<dyn DataFull>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .await?;
        let ret = VfsReadWrite { file };
        Ok(Box::new(ret))
    }
    pub async fn set_length(path: impl AsRef<Path>, size: u64) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path).await?
            .set_len(size).await?;
        Ok(())
    }
}

#[async_trait]
impl AsyncRead for VfsReadWrite {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = Pin::new(&mut self.file);
        this.poll_read(cx, buf)
    }
}

#[async_trait]
impl AsyncSeek for VfsReadWrite {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        let this = Pin::new(&mut self.file);
        this.start_seek(position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        let this = Pin::new(&mut self.file);
        this.poll_complete(cx)
    }
}

#[async_trait]
impl AsyncWrite for VfsReadWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = Pin::new(&mut self.file);
        this.poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = Pin::new(&mut self.file);
        this.poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = Pin::new(&mut self.file);
        this.poll_shutdown(cx)
    }
}

#[async_trait]
impl DataWrite for VfsReadWrite {
    async fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl DataRead for VfsReadWrite {}

impl DataReadSeek for VfsReadWrite {}

impl DataWriteSeek for VfsReadWrite {}

impl DataFull for VfsReadWrite {}