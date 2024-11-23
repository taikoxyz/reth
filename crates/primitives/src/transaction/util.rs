pub(crate) mod secp256k1 {
    use super::*;
    use crate::{keccak256, Address, Signature};
    pub(crate) use ::secp256k1::Error;
    use ::secp256k1::{
        ecdsa::{RecoverableSignature, RecoveryId},
        Message, PublicKey, SecretKey, SECP256K1,
    };
    use revm_primitives::{B256, U256};

    /// Recovers the address of the sender using secp256k1 pubkey recovery.
    ///
    /// Converts the public key into an ethereum address by hashing the public key with keccak256.
    ///
    /// This does not ensure that the `s` value in the signature is low, and _just_ wraps the
    /// underlying secp256k1 library.
    pub fn recover_signer_unchecked(sig: &[u8; 65], msg: &[u8; 32]) -> Result<Address, Error> {
        let address_bytes = k256::recover_signer_unchecked(sig, msg).map_err(|_| Error::InvalidSignature)?;
        Ok(Address::from_slice(&address_bytes))
    }

    /// Signs message with the given secret key.
    /// Returns the corresponding signature.
    pub fn sign_message(secret: B256, message: B256) -> Result<Signature, secp256k1::Error> {
        let sec = SecretKey::from_slice(secret.as_ref())?;
        let s = SECP256K1.sign_ecdsa_recoverable(&Message::from_digest(message.0), &sec);
        let (rec_id, data) = s.serialize_compact();

        let signature = Signature {
            r: U256::try_from_be_slice(&data[..32]).expect("The slice has at most 32 bytes"),
            s: U256::try_from_be_slice(&data[32..64]).expect("The slice has at most 32 bytes"),
            odd_y_parity: rec_id.to_i32() != 0,
        };
        Ok(signature)
    }

    /// Converts a public key into an ethereum address by hashing the encoded public key with
    /// keccak256.
    pub fn public_key_to_address(public: PublicKey) -> Address {
        // strip out the first byte because that should be the SECP256K1_TAG_PUBKEY_UNCOMPRESSED
        // tag returned by libsecp's uncompressed pubkey serialization
        let hash = keccak256(&public.serialize_uncompressed()[1..]);
        Address::from_slice(&hash[12..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{address, hex};

    #[test]
    fn sanity_ecrecover_call() {
        let sig = hex!("650acf9d3f5f0a2c799776a1254355d5f4061762a237396a99a0e0e3fc2bcd6729514a0dacb2e623ac4abd157cb18163ff942280db4d5caad66ddf941ba12e0300");
        let hash = hex!("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad");
        let out = address!("c08b5542d177ac6686946920409741463a15dddb");

        assert_eq!(secp256k1::recover_signer_unchecked(&sig, &hash), Ok(out));
    }
}

pub(crate) mod k256 {
    use k256::ecdsa::signature::digest::Digest;
    use k256::ecdsa::Error as K256_Error;
    use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

    use sha3::Keccak256;

    pub fn recover_signer_unchecked(
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<[u8; 20], K256_Error> {
        let mut signature = Signature::from_slice(&sig[..64])?;
        let mut recid_byte = sig[64];

        if let Some(sig_normalized) = signature.normalize_s() {
            signature = sig_normalized;
            recid_byte ^= 1;
        }
        let recid = RecoveryId::try_from(recid_byte).map_err(|_| K256_Error::default())?;

        let pk = VerifyingKey::recover_from_prehash(msg, &signature, recid)?;

        Ok(public_key_to_address(&pk))
    }

    fn public_key_to_address(public_key: &VerifyingKey) -> [u8; 20] {
        let uncompressed_pubkey = public_key.to_encoded_point(false);
        let public_key_bytes = &uncompressed_pubkey.as_bytes()[1..];

        let hash = Keccak256::digest(public_key_bytes);
        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..]);
        address
    }
}

#[cfg(test)]
mod k256_test {
    use super::*;
    use crate::hex;

    #[test]
    fn sanity_ecrecover_call() {
        let signature = hex!("650acf9d3f5f0a2c799776a1254355d5f4061762a237396a99a0e0e3fc2bcd6729514a0dacb2e623ac4abd157cb18163ff942280db4d5caad66ddf941ba12e0300");
        let message = hex!("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad");

        let signer_address = k256::recover_signer_unchecked(&signature, &message)
            .expect("Failed to recover signer address");
        // println!("Recovered address: {:?}", hex::encode(signer_address));
        assert_eq!(
            signer_address,
            hex::decode("c08b5542d177ac6686946920409741463a15dddb").unwrap().as_slice()
        );
    }
}
