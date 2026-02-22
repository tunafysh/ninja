use bytes::Bytes;
use futures_util::StreamExt;
use std::time::Instant;
use tokio::sync::mpsc;

pub struct Downloader {
    client: reqwest::Client,
}
impl Downloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn download<S: DownloadTarget>(
        &self,
        request: DownloadRequest,
        mut target: S,
        events: EventSender,
    ) -> Result<(), DownloadError> {
        let response = self
            .client
            .request(request.method, &request.url)
            .headers(request.headers)
            .send()
            .await?;

        let total = response.content_length();
        let _ = events.send(DownloadEvent::Started { total });

        let mut stream = response.bytes_stream();
        let start = Instant::now();
        let mut downloaded = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;

            // STREAM → TARGET (FILE, MEMORY, ETC.)
            target.write(&chunk).await?;

            // PROGRESS
            let elapsed = start.elapsed().as_secs_f64().max(0.001);
            let speed = downloaded as f64 / elapsed;
            let eta = total.map(|t| (t.saturating_sub(downloaded)) as f64 / speed);

            let _ = events.send(DownloadEvent::Progress {
                downloaded,
                total,
                speed_bps: speed,
                eta_secs: eta,
            });
        }

        target.finish().await?;
        let _ = events.send(DownloadEvent::Finished);

        Ok(())
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

//
// ===== Request =====
//

use reqwest::Method;
use reqwest::header::HeaderMap;

pub struct DownloadRequest {
    pub url: String,
    pub method: Method,
    pub headers: HeaderMap,
}

impl DownloadRequest {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: Method::GET,
            headers: HeaderMap::new(),
        }
    }
}

//
// ===== Events =====
//

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Started {
        total: Option<u64>,
    },
    Progress {
        downloaded: u64,
        total: Option<u64>,
        speed_bps: f64,
        eta_secs: Option<f64>,
    },
    Finished,
    Failed(String),
}

pub type EventSender = mpsc::UnboundedSender<DownloadEvent>;

//
// ===== Target (Sink) =====
//

#[async_trait::async_trait]
pub trait DownloadTarget: Send {
    async fn write(&mut self, chunk: &Bytes) -> std::io::Result<()>;
    async fn finish(&mut self) -> std::io::Result<()>;
}

//
// ===== File target (REAL streaming) =====
//

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct FileTarget {
    file: File,
}

impl FileTarget {
    pub async fn create(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        Ok(Self {
            file: File::create(path).await?,
        })
    }
}

#[async_trait::async_trait]
impl DownloadTarget for FileTarget {
    async fn write(&mut self, chunk: &Bytes) -> std::io::Result<()> {
        self.file.write_all(chunk).await
    }

    async fn finish(&mut self) -> std::io::Result<()> {
        self.file.flush().await
    }
}

//
// ===== Errors =====
//

#[derive(Debug)]
pub enum DownloadError {
    Http(reqwest::Error),
    Io(std::io::Error),
}

impl From<reqwest::Error> for DownloadError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e)
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
