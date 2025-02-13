use crate::key_server::{self, KeyServer};
use crate::secmod::Secmod;
use rlp::{Rlp, RlpStream};
use tiny_keccak::{Hasher, Keccak};
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("key_pool");
}

use crate::grpc::pb::{
    key_pool_service_server::KeyPoolService, BuiltinSigningKey, EcdsaSignature,
    GetEthereumAddressRequest, GetEthereumAddressResponse, HashFunction, SignDigestRequest,
    SignDigestResponse, SignEthereumTransactionRequest, SignEthereumTransactionResponse,
    SignMessageRequest, SignMessageResponse, SigningKey,
};

pub struct SignerServiceImpl<SM: Secmod> {
    pub key: std::sync::Arc<KeyServer<SM>>,
}

impl<SM: Secmod> SignerServiceImpl<SM> {
    async fn sign_ethereum_transaction(
        signing_key: &key_server::SecretPubKeyPair,
        transaction: &[u8],
    ) -> Result<Response<SignEthereumTransactionResponse>, Status> {
        // Parse RLP to determine if it's EIP-155
        let rlp = Rlp::new(transaction);
        let item_count =
            rlp.item_count().map_err(|_| Status::invalid_argument("decode message"))?;
        if item_count != 6 && item_count != 9 {
            return Err(Status::invalid_argument(format!(
                "invalid number of RLP items: {}; expeted 6 or 9",
                item_count,
            )));
        }
        let chain_id = if item_count == 9 {
            let chain_id =
                rlp.val_at::<u64>(6).map_err(|_| Status::invalid_argument("chain ID"))?;
            Some(chain_id)
        } else {
            None
        };
        let digest = Self::hash_message(transaction, HashFunction::Keccak256)?;

        let EcdsaSignature { r, s, is_y_odd, is_x_reduced: _ } =
            Self::sign_digest_internal(signing_key, &digest)?;

        // Compute v according to EIP-155 if chain_id is present
        let recovery_id = is_y_odd as u64;
        let v = if let Some(chain_id) = chain_id {
            (chain_id * 2 + 35) + recovery_id
        } else {
            27 + recovery_id
        };
        // Create signed transaction
        let mut stream = RlpStream::new_list(9);
        // first 6 elements (nonce, gasPrice, gasLimit, to, value, data)
        for i in 0..6 {
            let val = rlp.at(i).map_err(|_| Status::invalid_argument("decode element"))?;
            stream.append_raw(val.as_raw(), 1);
        }
        stream.append(&v);
        stream.append(&r);
        stream.append(&s);
        let response = SignEthereumTransactionResponse { tx_data: stream.out().to_vec() };
        Ok(Response::new(response))
    }

    fn sign_digest_internal(
        signing_key: &key_server::SecretPubKeyPair,
        digest: &[u8; 32],
    ) -> Result<EcdsaSignature, Status> {
        let key_server::EcdsaSignature { r, s, is_y_odd, is_x_reduced } =
            signing_key.ecdsa_sign_prehash(&digest).map_err(|x| Status::internal(x.to_string()))?;

        Ok(EcdsaSignature { r: r.to_vec(), s: s.to_vec(), is_y_odd, is_x_reduced })
    }

    fn hash_message(message: &[u8], hash_function: HashFunction) -> Result<[u8; 32], Status> {
        match hash_function {
            HashFunction::Sha256 => {
                use sha2::Digest;
                let mut hasher = sha2::Sha256::new();
                hasher.update(message);
                Ok(hasher.finalize().into())
            }
            HashFunction::Keccak256 => {
                let mut output = [0u8; 32];
                let mut hasher = Keccak::v256();
                hasher.update(message);
                hasher.finalize(&mut output);
                Ok(output)
            }
            HashFunction::Sha3256 => {
                let mut output = [0u8; 32];
                let mut hasher = tiny_keccak::Sha3::v256();
                hasher.update(message);
                hasher.finalize(&mut output);
                Ok(output)
            }
            HashFunction::Unspecified => Err(Status::invalid_argument("hash function unspecified")),
        }
    }

    fn signing_key(
        &self,
        signing_key: SigningKey,
        default: BuiltinSigningKey,
    ) -> Result<&key_server::SecretPubKeyPair, Status> {
        assert!(default != BuiltinSigningKey::Unspecified);
        let key_index = if signing_key.key_index as u32 == BuiltinSigningKey::Unspecified as u32 {
            default as u32
        } else {
            signing_key.key_index
        };
        if key_index == 0 {
            return Err(Status::invalid_argument("key_index must not be zero"));
        }
        // Note that key_index zero corresponds to BUILTIN_SIGNING_KEY_UNSPECIFIED.
        // Thus, the valid values for key_index are 1..N where N is as configured.
        let key_index = key_index - 1;
        if key_index as usize >= self.key.pairs.len() {
            return Err(Status::invalid_argument(format!(
                "key_index must not be greater than {}",
                self.key.pairs.len()
            )));
        }
        Ok(&self.key.pairs[key_index as usize])
    }
}

#[tonic::async_trait]
impl<SM: Secmod + 'static> KeyPoolService for SignerServiceImpl<SM> {
    async fn sign_digest(
        &self,
        request: Request<SignDigestRequest>,
    ) -> Result<Response<SignDigestResponse>, Status> {
        let request = request.into_inner();
        let signing_key = request.signing_key.unwrap_or_default();
        let signing_key = self.signing_key(signing_key, BuiltinSigningKey::ServiceResponse)?;
        let digest: [u8; 32] = request.digest.try_into().map_err(|x: Vec<u8>| {
            Status::invalid_argument(format!("digest must be 32 bytes - was {}", x.len()))
        })?;
        let ecdsa_signature = Self::sign_digest_internal(signing_key, &digest)?;
        let response = SignDigestResponse { signature: Some(ecdsa_signature) };
        Ok(Response::new(response))
    }

    async fn sign_message(
        &self,
        request: Request<SignMessageRequest>,
    ) -> Result<Response<SignMessageResponse>, Status> {
        let request = request.into_inner();
        let signing_key = request.signing_key.unwrap_or_default();
        let signing_key = self.signing_key(signing_key, BuiltinSigningKey::ServiceResponse)?;
        let hash_function = request.hash_function();
        let message = request.message;
        if message.len() > (1 << 20) {
            return Err(Status::invalid_argument("message too long"));
        }
        let digest = Self::hash_message(&message, hash_function)?;
        let mut ecdsa_signature = Self::sign_digest_internal(signing_key, &digest)?;
        let mut eth_format = Vec::new();
        eth_format.append(&mut ecdsa_signature.r);
        eth_format.append(&mut ecdsa_signature.s);
        eth_format.push(ecdsa_signature.is_y_odd as u8);
        let response = SignMessageResponse { signature: eth_format };
        Ok(Response::new(response))
    }

    async fn sign_ethereum_transaction(
        &self,
        request: Request<SignEthereumTransactionRequest>,
    ) -> Result<Response<SignEthereumTransactionResponse>, Status> {
        let request = request.into_inner();
        let signing_key = request.signing_key.unwrap_or_default();
        let signing_key = self.signing_key(signing_key, BuiltinSigningKey::Ethereum)?;
        let response = Self::sign_ethereum_transaction(signing_key, &request.tx_data).await?;
        Ok(response)
    }

    async fn get_ethereum_address(
        &self,
        request: Request<GetEthereumAddressRequest>,
    ) -> Result<Response<GetEthereumAddressResponse>, Status> {
        let request = request.into_inner();
        let signing_key = request.signing_key.unwrap_or_default();
        let signing_key = self.signing_key(signing_key, BuiltinSigningKey::Ethereum)?;
        let addr = signing_key.ethereum_address();
        let hex_addr = hex::encode(addr);
        let response = GetEthereumAddressResponse { ethereum_address: hex_addr };
        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use hex;
    use key_server::SecretPubKeyPair;
    use rlp::{Rlp, RlpStream};

    fn create_test_key() -> SecretPubKeyPair {
        let secret_key: [u8; 32] =
            hex::decode("4646464646464646464646464646464646464646464646464646464646464646")
                .unwrap()
                .try_into()
                .unwrap();
        let secret_key = k256::SecretKey::from_bytes(
            elliptic_curve::generic_array::GenericArray::from_slice(&secret_key),
        )
        .unwrap();
        SecretPubKeyPair::from_secret_key(secret_key)
    }

    //Magic numbers from https://eips.ethereum.org/EIPS/eip-155.
    #[tokio::test]
    async fn test_sign_eip155_transaction() {
        let signing_key = create_test_key();
        let transaction = hex::decode("ec098504a817c800825208943535353535353535353535353535353535353535880de0b6b3a764000080018080").unwrap();
        let result = SignerServiceImpl::<crate::nsm::Nsm>::sign_ethereum_transaction(
            &signing_key,
            &transaction,
        )
        .await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        let expected = hex::decode("f86c098504a817c800825208943535353535353535353535353535353535353535880de0b6b3a76400008025a028ef61340bd939bc2195fe537567866003e1a15d3c71ff63e1590620aa636276a067cbe9d8997f761aecb703304b3800ccf555c9f3dc64214b297fb1966a3b6d83").unwrap();
        assert_eq!(expected, response.tx_data);
        // Verify the signed transaction
        let rlp = Rlp::new(&response.tx_data);
        assert_eq!(rlp.item_count().unwrap(), 9);
        // Verify v follows EIP-155 format
        let v = rlp.val_at::<u64>(6).unwrap();
        assert!(v == 37);
        let r = rlp.val_at::<Vec<u8>>(7).unwrap();
        let s = rlp.val_at::<Vec<u8>>(8).unwrap();
        // decimal 18515461264373351373200002665853028612451056578545711640558177340181847433846
        let r_expect =
            hex::decode("28ef61340bd939bc2195fe537567866003e1a15d3c71ff63e1590620aa636276")
                .unwrap();
        assert_eq!(r, r_expect);
        // decimal 46948507304638947509940763649030358759909902576025900602547168820602576006531
        let s_expect =
            hex::decode("67cbe9d8997f761aecb703304b3800ccf555c9f3dc64214b297fb1966a3b6d83")
                .unwrap();
        assert_eq!(s, s_expect);
    }

    fn create_test_transaction(chain_id: Option<u64>) -> Vec<u8> {
        let mut stream = RlpStream::new();

        // If chain_id is present, create EIP-155 transaction
        if chain_id.is_some() {
            stream.begin_list(9);
        } else {
            stream.begin_list(6);
        }

        // Append transaction fields
        stream.append(&0u64); // nonce
        stream.append(&20_000_000_000u64); // gasPrice
        stream.append(&21000u64); // gasLimit
        stream.append(&hex::decode("d46e8dd67c5d32be8058bb8eb970870f07244567").unwrap()); // to
        stream.append(&1_000_000_000u64); // value
        stream.append(&Vec::<u8>::new()); // data

        // Append EIP-155 fields if needed
        if let Some(chain_id) = chain_id {
            stream.append(&chain_id);
            stream.append(&0u8);
            stream.append(&0u8);
        }

        stream.out().to_vec()
    }

    #[tokio::test]
    async fn test_sign_legacy_transaction() {
        let signing_key = create_test_key();
        let transaction = create_test_transaction(None);

        let result = SignerServiceImpl::<crate::nsm::Nsm>::sign_ethereum_transaction(
            &signing_key,
            &transaction,
        )
        .await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        // Verify the signed transaction
        let rlp = Rlp::new(&response.tx_data);
        assert_eq!(rlp.item_count().unwrap(), 9);
        // Verify v is either 27 or 28
        let v = rlp.val_at::<u64>(6).unwrap();
        assert!(v == 27 || v == 28);
        // Verify r and s are non-zero
        let r = rlp.val_at::<Vec<u8>>(7).unwrap();
        let s = rlp.val_at::<Vec<u8>>(8).unwrap();
        assert!(!r.is_empty() && !s.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_rlp() {
        let signing_key = create_test_key();
        let invalid_rlp = vec![0xc0]; // Empty RLP list
        let result = SignerServiceImpl::<crate::nsm::Nsm>::sign_ethereum_transaction(
            &signing_key,
            &invalid_rlp,
        )
        .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().code(), tonic::Code::InvalidArgument));
    }

    #[tokio::test]
    async fn test_invalid_item_count() {
        let signing_key = create_test_key();
        let mut stream = RlpStream::new_list(5); // Wrong number of items
        for _ in 0..5 {
            stream.append(&0u64);
        }
        let result = SignerServiceImpl::<crate::nsm::Nsm>::sign_ethereum_transaction(
            &signing_key,
            &stream.out(),
        )
        .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().code(), tonic::Code::InvalidArgument));
    }
}
