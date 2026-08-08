use std::{fmt::Display, ops::Deref, sync::Arc};

use image::DynamicImage;
use image_hasher::HasherConfig;

use crate::processor::Hash;

#[derive(Debug, serde::Deserialize, Clone)]
pub enum HashingMethodType {
    Mean,
}
impl HashingMethodType {
    pub fn hashing_method(&self) -> Box<dyn HashingMethod> {
        match self {
            Self::Mean => Box::new(Mean),
        }
    }
}

pub trait HashingMethod: Send + Sync + Display {
    fn run(&self, image: &DynamicImage) -> Hash;
}

#[derive(Default, Clone)]
pub struct HashingMethods {
    methods: Vec<Arc<dyn HashingMethod>>,
}
impl From<Vec<HashingMethodType>> for HashingMethods {
    fn from(value: Vec<HashingMethodType>) -> Self {
        let mut hashing_methods = Self::default();
        value
            .iter()
            .for_each(|h| hashing_methods.push_boxed(h.hashing_method()));
        hashing_methods
    }
}
impl From<&[HashingMethodType]> for HashingMethods {
    fn from(value: &[HashingMethodType]) -> Self {
        let mut hashing_methods = Self::default();
        value
            .iter()
            .for_each(|h| hashing_methods.push_boxed(h.hashing_method()));
        hashing_methods
    }
}
impl From<&Vec<HashingMethodType>> for HashingMethods {
    fn from(value: &Vec<HashingMethodType>) -> Self {
        let mut hashing_methods = Self::default();
        value
            .iter()
            .for_each(|h| hashing_methods.push_boxed(h.hashing_method()));
        hashing_methods
    }
}

impl HashingMethods {
    pub fn push<T>(&mut self, method: T)
    where
        T: HashingMethod + 'static,
    {
        self.methods.push(Arc::new(method));
    }

    pub fn push_boxed(&mut self, method: Box<dyn HashingMethod>) {
        self.methods.push(Arc::from(method));
    }
}

impl Deref for HashingMethods {
    type Target = [Arc<dyn HashingMethod>];
    fn deref(&self) -> &Self::Target {
        &self.methods[..]
    }
}

pub struct Mean;
impl HashingMethod for Mean {
    fn run(&self, image: &DynamicImage) -> Hash {
        let hasher = HasherConfig::new()
            .hash_alg(image_hasher::HashAlg::Mean)
            .to_hasher();
        let hash = hasher.hash_image(image);
        Hash::from(hash.into_inner())
    }
}
impl Display for Mean {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mean")
    }
}
