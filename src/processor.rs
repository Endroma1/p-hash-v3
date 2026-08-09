use std::sync::{Arc, mpsc};

use image::DynamicImage;
use rayon::iter::{ParallelBridge, ParallelIterator};
use uuid::Uuid;

use crate::{FetchError, HashingMethod, HashingMethods, Modification, Modifications};

pub struct Hash(Box<[u8]>);
impl Hash {
    pub fn new(hash: Box<[u8]>) -> Self {
        Self(hash)
    }
    pub fn into_inner(self) -> Box<[u8]> {
        self.0
    }
    pub fn inner(&self) -> &[u8] {
        &self.0
    }
}
impl From<Box<[u8]>> for Hash {
    fn from(value: Box<[u8]>) -> Self {
        Self(value)
    }
}

pub struct ParseResult {
    pub image: Arc<ImageResult>,
    pub modification: Arc<ModifiedImage>,
    pub hashing_method: Arc<dyn HashingMethod>,
    pub hash: Hash,
}

// Image type that should be input for processing images.
pub struct Image {
    pub image: DynamicImage,
    pub name: String,
    pub uuid: Uuid,
}

// Image type as result of processing images such that images are not loaded in memory when not needed.
pub struct ImageResult {
    pub name: String,
    pub uuid: Uuid,
}

pub struct ModifiedImage {
    pub modification: Arc<dyn Modification>,
    pub uuid: Uuid,
}
impl From<Image> for ImageResult {
    fn from(value: Image) -> Self {
        Self {
            name: value.name,
            uuid: value.uuid,
        }
    }
}

fn parse_image<'a>(
    image: Image,
    modifications: &'a Modifications,
    hashing_methods: &'a HashingMethods,
) -> impl Iterator<Item = ParseResult> + 'a {
    let Image { image, name, uuid } = image;
    let image_result = Arc::new(ImageResult { name, uuid });

    modifications.iter().cloned().flat_map(move |modification| {
        let modified_image = modification.run(&image);

        let modified_image_result = {
            let uuid = Uuid::new_v4();
            let modification = ModifiedImage {
                modification: modification.clone(),
                uuid,
            };
            Arc::new(modification)
        };

        let image_result = image_result.clone();

        hashing_methods.iter().cloned().map(move |hashing_method| {
            let hash = hashing_method.run(&modified_image);
            ParseResult {
                image: image_result.clone(),
                modification: modified_image_result.clone(),
                hashing_method,
                hash,
            }
        })
    })
}

/// Parses a series of images with rayon.
#[tracing::instrument(
    name = "Parse images in parallel with rayon",
    skip(images, modifications, hashing_methods),
    level = "debug"
)]
pub fn parse_image_rayon(
    images: impl Iterator<Item = Result<Image, FetchError>> + Send,
    modifications: &Modifications,
    hashing_methods: &HashingMethods,
) -> impl Iterator<Item = ParseResult> {
    let (tx, rx) = mpsc::channel();
    images
        .filter_map(|i| match i {
            Ok(i) => Some(i),
            Err(e) => {
                tracing::warn!("Failed to fetch an image: {}", e);
                None
            }
        })
        .par_bridge()
        .for_each(move |image| {
            let results = parse_image(image, modifications, hashing_methods);
            for result in results {
                println!("Sending result {}", result.image.name);
                if tx.send(result).is_err() {
                    tracing::error!("Receiver exited before all results could be sent");
                    break;
                }
            }
        });

    rx.into_iter()
}
