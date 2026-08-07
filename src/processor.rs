use std::sync::{Arc, mpsc};

use image::DynamicImage;
use rayon::iter::{ParallelBridge, ParallelIterator};

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
    pub image_name: String,
    pub modification: Arc<dyn Modification>,
    pub hashing_method: Arc<dyn HashingMethod>,
    pub hash: Hash,
}
pub struct Image {
    pub image: DynamicImage,
    pub name: String,
}

fn parse_image<'a>(
    image: Image,
    modifications: &'a Modifications,
    hashing_methods: &'a HashingMethods,
) -> impl Iterator<Item = ParseResult> + 'a {
    modifications.iter().cloned().flat_map(move |modification| {
        let modified_image = modification.run(&image.image);

        let image_name = image.name.clone();
        hashing_methods.iter().cloned().map(move |hashing_method| {
            let hash = hashing_method.run(&modified_image);
            ParseResult {
                image_name: image_name.clone(),
                modification: modification.clone(),
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
            },
        })
        .par_bridge()
        .for_each(move |image| {
            let results = parse_image(image, modifications, hashing_methods);
            for result in results {
                if tx.send(result).is_err() {
                    tracing::error!("Receiver exited before all results could be sent");
                    break;
                }
            }
        });

    rx.into_iter()
}

