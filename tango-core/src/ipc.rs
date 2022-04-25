use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub struct Client(std::sync::Arc<tokio::sync::Mutex<Inner>>);

struct Inner {
    writer: std::pin::Pin<Box<dyn tokio::io::AsyncWrite + Send + 'static>>,
    reader: std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send + 'static>>,
}

impl Client {
    pub fn new_from_stdio() -> Self {
        Client(std::sync::Arc::new(tokio::sync::Mutex::new(Inner {
            writer: Box::pin(tokio::io::stdout()),
            reader: Box::pin(tokio::io::stdin()),
        })))
    }

    pub async fn send(&self, req: tango_protos::ipc::ToSupervisor) -> anyhow::Result<()> {
        let mut inner = self.0.lock().await;
        let buf = req.encode_to_vec();
        inner.writer.write_u32_le(buf.len() as u32).await?;
        inner.writer.write_all(&buf).await?;
        inner.writer.flush().await?;
        Ok(())
    }

    pub async fn receive(&self) -> anyhow::Result<tango_protos::ipc::FromSupervisor> {
        let mut inner = self.0.lock().await;
        let size = inner.reader.read_u32_le().await? as usize;
        let mut buf = vec![0u8; size];
        inner.reader.read_exact(&mut buf).await?;
        let resp = tango_protos::ipc::FromSupervisor::decode(bytes::Bytes::from(buf))?;
        Ok(resp)
    }
}
