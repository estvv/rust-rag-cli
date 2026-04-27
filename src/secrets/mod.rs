#[cfg(feature = "keyring")]
pub mod store;

#[cfg(feature = "keyring")]
pub use store::KeyStore;

pub struct Secrets;

impl Secrets {
    #[cfg(feature = "keyring")]
    pub fn new() -> KeyStore {
        KeyStore::new()
    }

    #[cfg(not(feature = "keyring"))]
    pub fn new() {
        eprintln!("Warning: Keyring support not compiled in. Enable 'keyring-support' feature.");
    }
}
