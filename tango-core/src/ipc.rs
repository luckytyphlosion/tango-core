use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::game;

#[derive(Debug, serde::Serialize, serde::Deserialize, typescript_type_def::TypeDef)]
pub struct Args {
    pub window_title: String,
    pub rom_path: String,
    pub save_path: String,
    pub keymapping: Keymapping,
    pub match_settings: Option<MatchSettings>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, typescript_type_def::TypeDef)]
pub struct MatchSettings {
    pub rng_seed: [u8; 16],
    pub input_delay: u32,
    pub is_polite: bool,
    pub match_type: u16,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, typescript_type_def::TypeDef)]
pub struct Keymapping {
    up: String,
    down: String,
    left: String,
    right: String,
    a: String,
    b: String,
    l: String,
    r: String,
    select: String,
    start: String,
}

impl TryInto<game::Keymapping> for Keymapping {
    type Error = serde_plain::Error;

    fn try_into(self) -> Result<game::Keymapping, Self::Error> {
        Ok(game::Keymapping {
            up: serde_plain::from_str(&self.up)?,
            down: serde_plain::from_str(&self.down)?,
            left: serde_plain::from_str(&self.left)?,
            right: serde_plain::from_str(&self.right)?,
            a: serde_plain::from_str(&self.a)?,
            b: serde_plain::from_str(&self.b)?,
            l: serde_plain::from_str(&self.l)?,
            r: serde_plain::from_str(&self.r)?,
            select: serde_plain::from_str(&self.select)?,
            start: serde_plain::from_str(&self.start)?,
        })
    }
}

impl Args {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

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

    pub async fn send(&self, req: tango_protos::ipc::Outgoing) -> anyhow::Result<()> {
        let mut inner = self.0.lock().await;
        let buf = req.encode_to_vec();
        inner.writer.write_u32_le(buf.len() as u32).await?;
        inner.writer.write_all(&buf).await?;
        inner.writer.flush().await?;
        Ok(())
    }

    pub async fn receive(&self) -> anyhow::Result<tango_protos::ipc::Incoming> {
        let mut inner = self.0.lock().await;
        let size = inner.reader.read_u32_le().await? as usize;
        let mut buf = vec![0u8; size];
        inner.reader.read_exact(&mut buf).await?;
        let resp = tango_protos::ipc::Incoming::decode(bytes::Bytes::from(buf))?;
        Ok(resp)
    }
}
