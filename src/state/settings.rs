use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::PgConnectOptions;

use crate::{HashingMethodType, ModificationType};

pub struct Settings {
    images_n: std::sync::Mutex<u64>,
    pub hashing_methods: Vec<HashingMethodType>,
    pub modifications: Vec<ModificationType>,
}
impl Settings {
    pub fn expected_number_of_results(&self) -> u64 {
        *self.images_n.lock().unwrap()
            * self.hashing_methods.len() as u64
            * self.modifications.len() as u64
    }
    pub fn set_images_n(&self, value: u64) {
        *self.images_n.lock().unwrap() = value;
    }
    pub fn images_n(&self) -> u64 {
        *self.images_n.lock().unwrap()
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            images_n: std::sync::Mutex::new(100),
            hashing_methods: vec![HashingMethodType::Mean],
            modifications: vec![ModificationType::Blur],
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
