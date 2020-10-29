use std::io;

use futures::io::Cursor;
use futures::prelude::*;
use futures::lock::BiLock;
use futures::{AsyncReadExt, AsyncWriteExt};

use async_trait::async_trait;
use std::pin::Pin;
use std::task::{Context, Poll};


/// Read Trait for async/await
///
#[async_trait]
pub trait ReadEx: Send {
    async fn read2(&mut self, buf: &mut [u8]) -> Result<usize, io::Error>;
}

/// Write Trait for async/await
///
#[async_trait]
pub trait WriteEx: Send {
    async fn write2(&mut self, buf: &[u8]) -> Result<usize, io::Error>;
}

#[async_trait]
impl<T: AsyncRead + Unpin + Send> ReadEx for T {
    async fn read2(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let n = AsyncReadExt::read(self, buf).await?;
        Ok(n)
    }
}

#[async_trait]
impl<T: AsyncWrite + Unpin + Send> WriteEx for T {
    async fn write2(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        AsyncWriteExt::write(self, buf).await
    }
}

// Problematic: how to mark ReadHalf with proper lifetime??
pub struct ReadHalf<T> {
    handle: BiLock<T>,
    fut: Option<Pin<Box<dyn Future<Output=io::Result<usize>> + Send + Unpin + 'static>>>,
}

impl<T> ReadHalf<T> {
    pub fn new(lock: BiLock<T>) -> Self {
        ReadHalf {
            handle: lock,
            fut: None,
        }
    }
}

impl<T: ReadEx + Unpin> AsyncRead for ReadHalf<T> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let mut lock = futures::ready!(self.handle.poll_lock(cx));
        let t = &mut *lock;
        if self.fut.is_none() {
            self.fut = Some(Box::pin(t.read2(buf)));
        }
        let mut fut = self.fut.take().expect("must not be none");
        match fut.as_mut().poll(cx) {
            Poll::Pending => {
                self.fut = Some(fut);
                Poll::Pending
            },
            Poll::Ready(ret) => {
                Poll::Ready(ret)
            }
        }
    }
}


pub trait ReadExt2: ReadEx {
    fn split2(self) -> (ReadHalf<Self>, WriteHalf<Self>)
        where
            Self: Sized + WriteEx + Unpin,
    {
        let (a, b) = BiLock::new(self);
        (ReadHalf { handle: a, fut: None }, WriteHalf { handle: b })
    }
}

impl<R: ReadEx + ?Sized> ReadExt2 for R {}

/// The writable half of an object returned from `AsyncRead::split`.
pub struct WriteHalf<T> {
    handle: BiLock<T>,
}

impl<W: WriteEx + Unpin> AsyncWrite for WriteHalf<W> {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &[u8]) -> Poll<io::Result<usize>> {
        unimplemented!()
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unimplemented!()
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unimplemented!()
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////
struct Test(Cursor<Vec<u8>>);

#[async_trait]
impl ReadEx for Test {
    async fn read2(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.0.read(buf).await
    }
}

#[async_trait]
impl WriteEx for Test {
    async fn write2(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.0.write(buf).await
    }
}


fn main() {
    futures::executor::block_on(async {
        let rw = Test(Cursor::new(vec![1, 2, 3]));
        let (mut reader, _writer) = rw.split2();
        let mut output = [0u8; 3];
        let bytes = reader.read2(&mut output[..]).await.unwrap();

        assert_eq!(bytes, 3);
        assert_eq!(output, [1, 2, 3]);
    });
}
