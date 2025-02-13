use crate::config::SovereignConfig;
use crate::secmod::Secmod;
use anyhow::{anyhow, Context, Result};
use elliptic_curve::rand_core::{self};
use k256::ecdsa;
use k256::elliptic_curve::generic_array::typenum::Unsigned;
use std::sync::Arc;

#[derive(PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SecretKeyMaterial {
    pub cert_secret_key: [u8; <p256::NistP256 as elliptic_curve::Curve>::FieldBytesSize::USIZE],
    pub secret_keys: Vec<[u8; <k256::Secp256k1 as elliptic_curve::Curve>::FieldBytesSize::USIZE]>,
}

impl SecretKeyMaterial {
    pub fn generate_random<T>(num_keys: u32, rng: &mut T) -> Result<Self>
    where
        T: rand_core::RngCore,
        T: rand_core::CryptoRng,
    {
        let mut result = SecretKeyMaterial::default();
        rng.try_fill_bytes(&mut result.cert_secret_key)?;
        for _ in 0..num_keys {
            let mut tmp: [u8; <k256::Secp256k1 as elliptic_curve::Curve>::FieldBytesSize::USIZE] =
                [0; <k256::Secp256k1 as elliptic_curve::Curve>::FieldBytesSize::USIZE];
            rng.try_fill_bytes(&mut tmp)?;
            result.secret_keys.push(tmp);
        }
        Ok(result)
    }
}

#[derive(Clone)]
pub struct SecretPubKeyPair {
    pub secret_key: k256::SecretKey,
    pub public_key: k256::PublicKey,
    pub ecdsa_signing_key: ecdsa::SigningKey,
}

pub struct EcdsaSignature {
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub is_y_odd: bool,
    pub is_x_reduced: bool,
}

impl SecretPubKeyPair {
    pub fn ethereum_address(&self) -> [u8; 20] {
        use elliptic_curve::sec1::ToEncodedPoint;
        // Get uncompressed public key bytes and skip first byte (0x04)
        let binding = self.public_key.to_encoded_point(false);
        let pubkey_bytes = binding.as_bytes();
        let pubkey_without_prefix = &pubkey_bytes[1..];
        use tiny_keccak::Hasher;
        // Hash with Keccak-256
        let mut output = [0u8; 32];
        let mut hasher = tiny_keccak::Keccak::v256();
        hasher.update(pubkey_without_prefix);
        hasher.finalize(&mut output);

        // Take last 20 bytes
        let mut address = [0u8; 20];
        address.copy_from_slice(&output[12..32]);
        address
    }

    pub fn from_secret_key(k: k256::SecretKey) -> Self {
        let public_key = k.public_key();
        let ecdsa_signing_key = ecdsa::SigningKey::from(&k);
        Self { secret_key: k, public_key, ecdsa_signing_key }
    }

    pub fn ecdsa_sign_prehash(&self, prehash: &[u8; 32]) -> Result<EcdsaSignature> {
        use k256::ecdsa::signature::hazmat::PrehashSigner;
        let signing_key: &ecdsa::SigningKey = &self.ecdsa_signing_key;
        let (signature, recovery_id): (ecdsa::Signature, ecdsa::RecoveryId) =
            signing_key.sign_prehash(prehash)?;
        tracing::trace!("ECDSA: {}", recovery_id.to_byte());
        Ok(EcdsaSignature {
            r: signature.r().to_bytes().into(),
            s: signature.s().to_bytes().into(),
            is_y_odd: recovery_id.is_y_odd(),
            is_x_reduced: recovery_id.is_x_reduced(),
        })
    }
}

pub struct KeyServer<SM: Secmod> {
    pub config: SovereignConfig,
    pub metrics: Arc<crate::monitoring::Metrics>,
    pub attestor: SM::Attestor,
    pub cert_secret_key: p256::SecretKey,
    pub cert_secret_key_der: pki_types::PrivateKeyDer<'static>,
    pub cert_public_key_der: Vec<u8>,
    pub cert: rcgen::Certificate,
    pub pairs: Vec<SecretPubKeyPair>,
}

impl<SM: Secmod> KeyServer<SM> {
    pub fn extract_secret_key_material(&self) -> SecretKeyMaterial {
        let cert_secret_key = self.cert_secret_key.to_bytes().into();
        let mut secret_keys = Vec::new();
        for k in self.pairs.iter() {
            secret_keys.push(k.secret_key.to_bytes().into());
        }
        SecretKeyMaterial { cert_secret_key, secret_keys }
    }

    pub fn new(
        attestor: SM::Attestor,
        config: SovereignConfig,
        key_material: SecretKeyMaterial,
    ) -> Result<Self> {
        use elliptic_curve::generic_array::GenericArray;

        let mut pairs = Vec::new();
        for k in key_material.secret_keys {
            let secret_key = k256::SecretKey::from_bytes(GenericArray::from_slice(&k))
                .context("failed to create secret key")?;
            let pair = SecretPubKeyPair::from_secret_key(secret_key);
            pairs.push(pair);
        }

        let cert_secret_key =
            p256::SecretKey::from_bytes(GenericArray::from_slice(&key_material.cert_secret_key))
                .context("failed to create secret key #2")?;
        use p256::pkcs8::EncodePrivateKey;
        let cert_pkcs8_der =
            cert_secret_key.to_pkcs8_der().context("failed to convert P256 key to PKCS8")?;
        let cert_private_key_der =
            pki_types::PrivatePkcs8KeyDer::from(cert_pkcs8_der.as_bytes().to_vec());

        let key_pair = rcgen::KeyPair::from_pkcs8_der_and_sign_algo(
            &cert_private_key_der,
            &rcgen::PKCS_ECDSA_P256_SHA256,
        )
        .map_err(|e| anyhow!("failed to create key pair: {}", e))?;

        let cert_public_key_der = key_pair.public_key_der();

        let mut subject_alt_names = config.alt_names.clone();
        subject_alt_names.push("localhost".to_string());
        subject_alt_names.dedup();
        let cert = rcgen::CertificateParams::new(subject_alt_names)
            .map_err(|e| anyhow!("failed to create certificate: {}", e))?
            .self_signed(&key_pair)
            .map_err(|e| anyhow!("failed to sign certificate: {}", e))?;

        let cert_secret_key_der = pki_types::PrivateKeyDer::from(cert_private_key_der);

        let metrics = Arc::new(crate::monitoring::Metrics::new());
        Ok(KeyServer {
            config,
            metrics,
            attestor,
            cert_secret_key,
            cert_secret_key_der,
            cert_public_key_der,
            cert,
            pairs,
        })
    }
}

#[cfg(test)]
// #[cfg(feature = "test-utils")]
mod tests {

    use super::*;
    use elliptic_curve::rand_core::OsRng;
    use ethereum_tx_sign::LegacyTransaction;
    use ethereum_tx_sign::Transaction;

    // Ensure that the generated ECDSa signature is consistent
    // with anohter crate `ethereum_tx_sign` which itself has an extensive test suite.
    #[tokio::test]
    async fn test_sign_eth() -> Result<()> {
        for k in 1..10 {
            let new_transaction = LegacyTransaction {
                chain: 1,
                nonce: 0,
                to: Some([0; 20]),
                value: 1675538,
                gas_price: 250,
                gas: 21000,
                data: vec![1, 2, 3, k],
            };
            let sec_k = k256::SecretKey::random(&mut OsRng);
            let sec = SecretPubKeyPair::from_secret_key(sec_k.clone());
            let hash = new_transaction.hash();
            let signed_hash = sec.ecdsa_sign_prehash(&hash)?;
            let ecdsa = new_transaction
                .ecdsa(&sec_k.to_bytes().as_slice())
                .map_err(|x| anyhow!("ecdsa {:?}", x))?;
            assert_eq!(signed_hash.r.to_vec(), ecdsa.r);
            assert_eq!(signed_hash.s.to_vec(), ecdsa.s);
            assert_eq!(signed_hash.is_y_odd as u64 + 37, ecdsa.v);
        }
        Ok(())
    }
}
