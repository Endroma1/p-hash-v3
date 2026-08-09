use std::{
    fmt::Display,
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::Context;
use fake::{Fake, faker::picsum};
use image::{DynamicImage, ImageReader};
use reqwest::Url;
use tokio::fs::{DirEntry, read_dir};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::Image;

#[derive(serde::Deserialize, Clone, Debug)]
pub enum Fetcher {
    Local { path: PathBuf },
    Picsum { images_n: u64 },
}
impl Fetcher {
    pub async fn execute(&self) -> Result<ReceiverStream<Result<Image, FetchError>>, FetchError> {
        match self {
            Self::Local { path } => fetch_images_local(path).await,
            Self::Picsum { images_n } => fetch_images_picsum(*images_n).await,
        }
    }
}

pub async fn fetch_images_local(
    dir_path: &Path,
) -> Result<ReceiverStream<Result<Image, FetchError>>, FetchError> {
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let mut dir = read_dir(&dir_path)
        .await
        .context("Failed to read directory from input path")?;

    while let Some(entry) = dir.next_entry().await.context("Could not get next entry")? {
        let sender = tx.clone();
        tokio::spawn(async move {
            let image = parse_entry(entry).await;
            if sender.send(image).await.is_err() {
                tracing::error!("Receiver dropped before expected")
            }
        });
    }
    drop(tx);

    Ok(ReceiverStream::new(rx))
}
async fn parse_entry(entry: DirEntry) -> Result<Image, FetchError> {
    if entry
        .file_type()
        .await
        .context("Could not get file_type of entry")?
        .is_file()
    {
        let image = read_image(&entry.path()).await?;
        let name = entry
            .file_name()
            .to_str()
            .map(|s| String::from(s))
            .context("Could not get file_name for entry")?;

        let uuid = Uuid::new_v4();

        let image = Image { image, name, uuid};
        Ok(image)
    } else {
        Err(FetchError::UnexpectedError(
            NotADirectoryError(entry.path()).into(),
        ))
    }
}

async fn read_image(path: &Path) -> Result<DynamicImage, ImageParseError> {
    let image = ImageReader::open(path)
        .context("Could not read image")?
        .with_guessed_format()
        .context("Could not guess format of image")?
        .decode()
        .context("Could not decode image")?;
    Ok(image)
}

#[tracing::instrument(name = "Starting image fetchers")]
pub async fn fetch_images_picsum(
    images_n: u64,
) -> Result<ReceiverStream<Result<Image, FetchError>>, FetchError> {
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    for _ in 0..images_n {
        let tx = tx.clone();
        tokio::spawn(async move {
            let image = fetch_picsum_image()
                .await
                .map_err(|e| FetchError::ImageParseError(e));
            if tx.send(image).await.is_err() {
                tracing::error!("Receiver exited before all images could be sent");
                return;
            };
        });
    }
    drop(tx);
    Ok(ReceiverStream::new(rx))
}

#[tracing::instrument(name = "Fetcing image from picsum")]
pub async fn fetch_picsum_image() -> Result<Image, ImageParseError> {
    let picsum: String = picsum::en::Image().fake();
    let url = Url::parse(&picsum).unwrap();
    let image_raw = reqwest::get(url.clone())
        .await
        .context("Could not connect to Picsum")?
        .error_for_status()
        .context("Error from picsum server")?
        .bytes()
        .await
        .context("Failed to get bytes from picsum response")?;
    let image = ImageReader::new(Cursor::new(image_raw))
        .with_guessed_format()
        .context("Could not guess format of picsum image")?
        .decode()
        .context("Could not decode picsum image")?;
    let uuid = Uuid::new_v4();
    Ok(Image {
        image,
        name: url.to_string(),
        uuid
    })
}

#[derive(Debug)]
pub enum FetchError {
    // Error that fails during parsing and should just skip to next image.
    ImageParseError(ImageParseError),
    // Error that is unexpected and should stop the entire process.
    UnexpectedError(anyhow::Error),
}
impl Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ImageParseError(ImageParseError(e)) => write!(f, "Failed to parse image: {}", e),
            Self::UnexpectedError(e) => write!(f, "Unexpected error when parsing image: {}", e),
        }
    }
}
impl From<anyhow::Error> for FetchError {
    fn from(value: anyhow::Error) -> Self {
        Self::UnexpectedError(value)
    }
}
impl From<ImageParseError> for FetchError {
    fn from(value: ImageParseError) -> Self {
        Self::ImageParseError(value)
    }
}
#[derive(Debug)]
pub struct ImageParseError(anyhow::Error);
impl From<anyhow::Error> for ImageParseError {
    fn from(value: anyhow::Error) -> Self {
        ImageParseError(value)
    }
}
impl Display for ImageParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse image: {}", self.0)
    }
}
#[derive(Debug)]
pub struct NotADirectoryError(PathBuf);
impl Display for NotADirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} is not a directory when expected", self.0)
    }
}
impl std::error::Error for NotADirectoryError {}
