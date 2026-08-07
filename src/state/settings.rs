use crate::{HashingMethodType, ModificationType};

pub struct Settings {
    pub images_n: std::sync::Mutex<u64>,
    pub hashing_methods: Vec<HashingMethodType>,
    pub modifications: Vec<ModificationType>,
}
impl Settings {
    pub fn expected_number_of_results(&self) -> u64 {
        *self.images_n.lock().unwrap()
            * self.hashing_methods.len() as u64
            * self.modifications.len() as u64
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            images_n: std::sync::Mutex::new(100),
            hashing_methods: vec![HashingMethodType::Mean],
            modifications: vec![ModificationType::Blur]
        }
    }
}
