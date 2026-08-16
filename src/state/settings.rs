use std::fs::read_dir;

use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::PgConnectOptions;
use uuid::Uuid;

use crate::{Fetcher, HashingMethodType, ModificationType};

#[derive(Debug, Clone)]
pub struct Settings {
    pub hashing_methods: Vec<HashingMethodType>,
    pub modifications: Vec<ModificationType>,
    pub fetcher: Fetcher,
    pub send_events: bool,
    pub run_name: String,
}
impl Settings {
    pub fn disable_event_loop(&mut self) -> &mut Self {
        self.send_events = false;
        self
    }
    pub fn fetcher(&mut self, fetcher: Fetcher) -> &mut Self {
        self.fetcher = fetcher;
        self
    }
    pub fn hashing_methods(&mut self, methods: Vec<HashingMethodType>) -> &mut Self {
        self.hashing_methods = methods;
        self
    }
    pub fn modifications(&mut self, methods: Vec<ModificationType>) -> &mut Self {
        self.modifications = methods;
        self
    }
    pub fn run_name(&mut self, name: String) -> &mut Self {
        self.run_name = name;
        self
    }
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.hashing_methods.len() == 0 {
            return Err(ValidationError::NohashingMethodsSelected);
        }
        if self.modifications.len() == 0 {
            return Err(ValidationError::NoModificationsSelected);
        }
        if let Fetcher::Picsum { images_n } = self.fetcher {
            if images_n <= 0 {
                return Err(ValidationError::InvalidNumberOfImages(images_n));
            }
        }
        Ok(())
    }
    pub fn expected_number_of_results(&self) -> u64 {
        let base = self.hashing_methods.len() as u64 * self.modifications.len() as u64;
        match &self.fetcher {
            Fetcher::Picsum { images_n } => images_n * base,
            Fetcher::Local { path } => {
                // Very scuffed, should validate path and skip entries that contains error.
                let mut total: u64 = 0;
                let mut dir = read_dir(path).expect("Could not read directory");
                while let Some(entry) = dir.next() {
                    if entry.unwrap().file_type().unwrap().is_file() {
                        total += 1
                    }
                }
                total
            }
        }
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hashing_methods: vec![HashingMethodType::Mean],
            modifications: vec![ModificationType::Blur],
            fetcher: Fetcher::Picsum { images_n: 10 },
            send_events: true,
            run_name: Uuid::new_v4().into(),
        }
    }
}

pub struct DatabaseSettings {
    pub user: String,
    pub password: SecretString,
    pub database: String,
    pub port: u16,
    pub host: String,
}

impl DatabaseSettings {
    pub fn connection_options(&self) -> PgConnectOptions {
        PgConnectOptions::default()
            .host(&self.host)
            .username(&self.user)
            .password(&self.password.expose_secret())
            .database(&self.database)
            .port(self.port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("No hashing methods were selected")]
    NohashingMethodsSelected,
    #[error("No modifications were selected")]
    NoModificationsSelected,
    #[error("Number of images to be fetched was {0}. Expected >0")]
    InvalidNumberOfImages(u64),
}
