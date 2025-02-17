use crate::result::{CryptoError, DatabaseIntegrityError, Error, Result};

use aes::Aes256;
use block_modes::{block_padding::Pkcs7, BlockMode, Cbc};
use cipher::{generic_array::GenericArray, StreamCipher};
use salsa20::{cipher::NewCipher, Salsa20};

pub(crate) trait Cipher {
    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>>;
    fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>>;
}

type Aes256Cbc = Cbc<Aes256, Pkcs7>;
pub(crate) struct AES256Cipher {
    key: Vec<u8>,
    iv: Vec<u8>,
}

impl AES256Cipher {
    pub(crate) fn new(key: &[u8], iv: &[u8]) -> Result<Self> {
        Ok(AES256Cipher {
            key: Vec::from(key),
            iv: Vec::from(iv),
        })
    }
}

impl Cipher for AES256Cipher {
    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Cbc::new_from_slices(&self.key, &self.iv)
            .map_err(|e| Error::from(DatabaseIntegrityError::from(CryptoError::from(e))))?;

        let res = cipher
            .decrypt_vec(&ciphertext)
            .map_err(|e| Error::from(DatabaseIntegrityError::from(CryptoError::from(e))))?;

        Ok(res)
    }
    fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Cbc::new_from_slices(&self.key, &self.iv)
            .map_err(|e| Error::from(DatabaseIntegrityError::from(CryptoError::from(e))))?;

        Ok(cipher.encrypt_vec(data))
    }
}

type TwofishCbc = Cbc<twofish::Twofish, Pkcs7>;
pub(crate) struct TwofishCipher {
    key: Vec<u8>,
    iv: Vec<u8>,
}

impl TwofishCipher {
    pub(crate) fn new(key: &[u8], iv: &[u8]) -> Result<Self> {
        Ok(TwofishCipher {
            key: Vec::from(key),
            iv: Vec::from(iv),
        })
    }
}

impl Cipher for TwofishCipher {
    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let cipher = TwofishCbc::new_from_slices(&self.key, &self.iv)
            .map_err(|e| Error::from(DatabaseIntegrityError::from(CryptoError::from(e))))?;

        let res = cipher
            .decrypt_vec(&ciphertext)
            .map_err(|e| Error::from(DatabaseIntegrityError::from(CryptoError::from(e))))?;

        Ok(res)
    }
    fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = TwofishCbc::new_from_slices(&self.key, &self.iv)
            .map_err(|e| Error::from(DatabaseIntegrityError::from(CryptoError::from(e))))?;
        Ok(cipher.encrypt_vec(data))
    }
}

pub(crate) struct Salsa20Cipher {
    cipher: salsa20::Salsa20,
}

impl Salsa20Cipher {
    pub(crate) fn new(key: &[u8]) -> Result<Self> {
        let key = GenericArray::from_slice(key);
        let iv = GenericArray::from([0xE8, 0x30, 0x09, 0x4B, 0x97, 0x20, 0x5D, 0x2A]);

        Ok(Salsa20Cipher {
            cipher: Salsa20::new(&key, &iv),
        })
    }
}

impl Cipher for Salsa20Cipher {
    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let mut buffer = Vec::from(ciphertext);
        self.cipher.apply_keystream(&mut buffer);
        Ok(buffer)
    }
    fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.decrypt(data)
    }
}

pub(crate) struct ChaCha20Cipher {
    cipher: chacha20::ChaCha20,
}

impl ChaCha20Cipher {
    /// Create as an inner cipher by splitting up a SHA512 hash
    pub(crate) fn new(key: &[u8]) -> Result<Self> {
        let iv = crate::crypt::calculate_sha512(&[key])?;

        let key = GenericArray::from_slice(&iv[0..32]);
        let nonce = GenericArray::from_slice(&iv[32..44]);

        Ok(ChaCha20Cipher {
            cipher: chacha20::ChaCha20::new(&key, &nonce),
        })
    }

    /// Create as an outer cipher by separately-specified key and iv
    pub(crate) fn new_key_iv(key: &[u8], iv: &[u8]) -> Result<Self> {
        Ok(ChaCha20Cipher {
            cipher: chacha20::ChaCha20::new_from_slices(&key, &iv)
                .map_err(|e| Error::from(DatabaseIntegrityError::from(CryptoError::from(e))))?,
        })
    }
}

impl Cipher for ChaCha20Cipher {
    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let mut buffer = Vec::from(ciphertext);
        self.cipher.apply_keystream(&mut buffer);
        Ok(buffer)
    }
    fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.decrypt(data)
    }
}

pub(crate) struct PlainCipher;
impl PlainCipher {
    pub(crate) fn new(_: &[u8]) -> Result<Self> {
        Ok(PlainCipher)
    }
}
impl Cipher for PlainCipher {
    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        Ok(Vec::from(ciphertext))
    }
    fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        println!("Plain");
        Ok(Vec::from(data))
    }
}

#[test]
fn test_decrypt_encrypt_plain() -> Result<()> {
    let data = "hi this is a test";
    let bdata: Vec<u8> = data.as_bytes().to_vec();
    let encrypted = PlainCipher.encrypt(&bdata)?;
    let decrypted = PlainCipher.decrypt(&encrypted)?;
    assert_eq!(bdata, decrypted);
    Ok(())
}
