use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Context, Result};
use ed25519_dalek::SigningKey;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use std::fs;
use std::path::Path;

const NONCE_SIZE: usize = 12; // 12 bytes -> 96 bit -> 2^96

fn hash_password(password: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), b"", 200_000, &mut key);
    key
}

pub fn generate_wallet(wallet_path: &Path, password: &str) -> Result<()> {
    if wallet_path.exists() {
        return Err(anyhow!("The provided wallet path already exists"));
    }

    let mut csprng = rand::thread_rng();
    let secret_key = SigningKey::generate(&mut csprng);
    let mut nonce = [0u8; NONCE_SIZE];
    csprng.fill_bytes(&mut nonce);

    let key = hash_password(password);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let wallet = match cipher.encrypt(Nonce::from_slice(&nonce), secret_key.to_bytes().as_ref()) {
        Ok(bytes) => bytes,
        Err(_) => return Err(anyhow!("Failed to encrypt the wallet")),
    };

    fs::write(wallet_path, [nonce.as_slice(), &wallet].concat())
        .context("Failed to save your wallet")?;

    Ok(())
}

pub fn open_wallet(wallet_path: &Path, password: &str) -> Result<SigningKey> {
    let wallet = fs::read(wallet_path).context("Failed to read the wallet")?;
    let key = hash_password(password);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let secret_key = match cipher.decrypt(
        Nonce::from_slice(&wallet[..NONCE_SIZE]),
        wallet[NONCE_SIZE..].as_ref(),
    ) {
        Ok(data) => SigningKey::from_bytes(&data.try_into().unwrap()),
        Err(_) => return Err(anyhow!("Failed to encrypt the wallet")),
    };

    Ok(secret_key)
}
