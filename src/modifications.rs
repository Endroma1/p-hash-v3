use std::{fmt::Display, ops::Deref, sync::Arc};

use image::{DynamicImage, imageops::blur};

#[derive(Debug, serde::Deserialize, Clone)]
pub enum ModificationType {
    Blur,
}
impl ModificationType {
    pub fn modification(&self) -> Box<dyn Modification> {
        match self {
            Self::Blur => Box::new(Blur),
        }
    }
}

pub trait Modification: Send + Sync + Display {
    fn run(&self, image: &DynamicImage) -> DynamicImage;
}

#[derive(Default, Clone)]
pub struct Modifications {
    methods: Vec<Arc<dyn Modification>>,
}
impl From<Vec<ModificationType>> for Modifications{
    fn from(value: Vec<ModificationType>) -> Self {
        let mut modifications = Modifications::default();
        value.iter().for_each(|m| modifications.push_boxed(m.modification()));
        modifications
    }
}
impl Modifications {
    pub fn push<T>(&mut self, method: T)
    where
        T: Modification + 'static,
    {
        self.methods.push(Arc::new(method));
    }

    pub fn push_boxed(&mut self, method: Box<dyn Modification>) {
        self.methods.push(Arc::from(method));
    }
}
impl Deref for Modifications {
    type Target = [Arc<dyn Modification>];
    fn deref(&self) -> &Self::Target {
        &self.methods[..]
    }
}

pub struct Blur;

impl Modification for Blur {
    fn run(&self, image: &DynamicImage) -> DynamicImage {
        blur(image, 0.5).into()
    }
}
impl Display for Blur{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "blur")
    }
}

