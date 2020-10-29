use std::io;
use futures::prelude::*;

use std::pin::Pin;
use std::task::{Context, Poll};
use futures::FutureExt;


struct MyT;

impl MyT {
    async fn read2(&mut self, _buf: &mut [u8]) -> Result<usize, io::Error> {
        Ok(0)
    }
    async fn read(&mut self) -> Result<usize, io::Error> {
        Ok(0)
    }
}

// Problematic: how to mark ReadHalf with proper lifetime??
pub struct ReadHalf<MyT> {
    handle: Option<MyT>,
    fut: Option<Pin<Box<dyn Future<Output=io::Result<usize>> + Send + Sync>>>,
}

impl<MyT> ReadHalf<MyT> {
    pub fn new(lock: MyT) -> Self {
        ReadHalf {
            handle: Some(lock),
            fut: None,
        }
    }
}

impl AsyncRead for ReadHalf<MyT> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>>
    {
        if self.fut.is_none() {

            let mut aa = self.handle.take().unwrap();
            self.fut = Some(Box::pin(async move {
                aa.read().await
            }));
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

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////
fn main() {

}
