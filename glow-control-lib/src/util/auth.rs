use crate::util::rc4::Rc4;
use anyhow::Result;
use rand::RngCore;
use sha1::Digest;
use sha1::Sha1;

const SHARED_KEY_CHALLENGE: &[u8] = b"evenmoresecret!!";
pub struct Auth;

impl Auth {
    pub fn make_challenge_response(challenge: &[u8], mac_address: &str) -> Result<String> {
        let derived_key = Self::derive_key(SHARED_KEY_CHALLENGE, mac_address);
        let mut rc4_cipher = Rc4::new(&derived_key);
        let mut encrypted_challenge = challenge.to_vec();
        rc4_cipher.apply_keystream(&mut encrypted_challenge);

        let mut hasher = Sha1::new();
        hasher.update(&encrypted_challenge);
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }

    // Helper function to convert a MAC address string to bytes
    pub fn mac_to_bytes(mac: &str) -> Vec<u8> {
        mac.split(':')
            .map(|part| u8::from_str_radix(part, 16).unwrap())
            .collect()
    }

    // Function to derive the key from the shared key and MAC address
    pub fn derive_key(shared_key: &[u8], mac_address: &str) -> Vec<u8> {
        let mac_bytes = Self::mac_to_bytes(mac_address);
        let mut derived_key = Vec::new();
        for (i, &byte) in shared_key.iter().enumerate() {
            derived_key.push(byte ^ mac_bytes[i % mac_bytes.len()]);
        }
        derived_key
    }

    pub fn generate_challenge() -> Vec<u8> {
        let mut challenge = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut challenge);
        challenge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_challenge() {
        let challenge = Auth::generate_challenge();
        assert_eq!(challenge.len(), 32);
    }
}
