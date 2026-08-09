use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::PgConnectOptions;

use crate::{Fetcher, HashingMethodType, ModificationType};

#[derive(Debug, Clone)]
pub struct Settings {
    pub images_n: u64,
    pub hashing_methods: Vec<HashingMethodType>,
    pub modifications: Vec<ModificationType>,
    pub fetcher: Fetcher,
}
impl Settings {
    pub fn expected_number_of_results(&self) -> u64 {
        self.images_n
            * self.hashing_methods.len() as u64
            * self.modifications.len() as u64
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            images_n: 100,
            hashing_methods: vec![HashingMethodType::Mean],
            modifications: vec![ModificationType::Blur],
            fetcher: Fetcher::Picsum { images_n: 10 },
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
