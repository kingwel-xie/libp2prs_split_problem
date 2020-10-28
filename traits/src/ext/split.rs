// Copyright 2020 Netwarps Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use std::{future::Future, io, task::Poll};
use std::pin::Pin;
use std::task::Context;
use futures::{AsyncRead, AsyncWrite};
use futures::lock::BiLock;

use super::{ReadEx, WriteEx};

//
// pub struct ReadHalf<T> {
//     handle: BiLock<T>,
// }
//
// // Problematic 1:  t.read2() is a future, which might return Pending when being polled,
// // therefore, we drop an unfinished future in this case...
// impl<T: ReadEx + Unpin> AsyncRead for ReadHalf<T> {
//     fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
//         let mut lock = futures::ready!(self.handle.poll_lock(cx));
//         let t = &mut *lock;
//
//         let fut = t.read2(buf);
//         futures::pin_mut!(fut);
//         let ret = futures::ready!(fut.poll(cx));
//         Poll::Ready(ret)
//     }
// }
//
// pub(super) fn split<T>(t: T) -> (ReadHalf<T>, WriteHalf<T>)
//     where
//         T: ReadEx + WriteEx + Unpin,
// {
//     let (a, b) = BiLock::new(t);
//     (ReadHalf { handle: a }, WriteHalf { handle: b })
// }



// Problematic 2: a unfinished future should be remembered so that it can be polled again
// But how to mark ReadHalf2 with proper lifetime??
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

pub(super) fn split<T>(t: T) -> (ReadHalf<T>, WriteHalf<T>)
    where
        T: ReadEx + WriteEx + Unpin,
{
    let (a, b) = BiLock::new(t);
    (ReadHalf { handle: a, fut: None }, WriteHalf { handle: b })
}



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
