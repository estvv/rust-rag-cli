// src/secrets/store.rs

use keyring::Entry;
use std::error::Error;

pub struct KeyStore {
    service: String,
}

impl KeyStore {
    pub fn new() -> Self {
        Self {
            service: "rust-rag-cli".to_string(),
        }
    }

    pub fn with_service(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
        let entry = Entry::new(&self.service, key)?;
        entry.set_password(value)?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<String, Box<dyn Error>> {
        let entry = Entry::new(&self.service, key)?;
        let password = entry.get_password()?;
        Ok(password)
    }

    pub fn delete(&self, key: &str) -> Result<(), Box<dyn Error>> {
        let entry = Entry::new(&self.service, key)?;
        entry.delete_password()?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(Vec::new())
    }
}

impl Default for KeyStore {
    fn default() -> Self {
        Self::new()
    }
}
