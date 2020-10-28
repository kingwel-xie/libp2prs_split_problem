use futures::io::{self, AsyncReadExt, AsyncWriteExt, Cursor};
use async_trait::async_trait;

use libp2prs_traits::{ReadEx, WriteEx};
use libp2prs_traits::ReadExt2;


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

    async fn flush2(&mut self) -> Result<(), io::Error> {
        self.0.flush().await
    }

    async fn close2(&mut self) -> Result<(), io::Error> {
        self.0.close().await
    }
}


fn main() {
    println!("Hello, world!");

    futures::executor::block_on(async {
        let rw = Test(Cursor::new(vec![1, 2, 3]));

        let (mut reader, _writer) = rw.split2();

        let mut output = [0u8; 3];
        let bytes = reader.read2(&mut output[..]).await.unwrap();

        assert_eq!(bytes, 3);
        assert_eq!(output, [1, 2, 3]);

    });

}
