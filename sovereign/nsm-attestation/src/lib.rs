// This code verifies an AWS attestation document.
//
// Due to poor design of the AWS attestation document, the code is somewhat convoluted.
// The problem with the AWS attestation document design is that it doesn't adhere to
// the layering principle, i.e., one has to decode the CBOR document contained inside
// the COSE envelope before being able to verify the signature on the COSE envelope.
use anyhow::{anyhow, bail, Result};
use aws_nitro_enclaves_cose::CoseSign1;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct NitroAttestationDocument {
    pub module_id: String,
    pub digest: String,
    pub timestamp: u64,
    pub pcrs: std::collections::HashMap<u8, ByteBuf>,
    pub certificate: ByteBuf,
    pub cabundle: Vec<ByteBuf>,
    pub public_key: Option<ByteBuf>,
    pub user_data: Option<ByteBuf>,
    pub nonce: Option<ByteBuf>,
}

use openssl::x509::X509;

#[cfg(feature = "test-utils")]
use openssl::{
    asn1::Asn1Time,
    hash::MessageDigest,
    pkey::{PKey, Private},
    x509::X509NameBuilder,
};

#[cfg(feature = "test-utils")]
lazy_static::lazy_static! {

    pub static ref TEST_ROOT_CA_KEY: PKey<Private> = {
        let ec_group = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::X9_62_PRIME256V1).unwrap();
        let ec_key = openssl::ec::EcKey::generate(&ec_group).unwrap();
        PKey::from_ec_key(ec_key).unwrap()
    };

    pub static ref TEST_ROOT_CA_CERT: X509 = {
        let mut x509_name = X509NameBuilder::new().unwrap();
        x509_name.append_entry_by_text("C", "US").unwrap();
        x509_name.append_entry_by_text("O", "Test Organization").unwrap();
        x509_name.append_entry_by_text("CN", "Test Root CA").unwrap();
        let x509_name = x509_name.build();

        let mut cert_builder = X509::builder().unwrap();
        cert_builder.set_version(2).unwrap();
        cert_builder.set_subject_name(&x509_name).unwrap();
        cert_builder.set_issuer_name(&x509_name).unwrap();
        cert_builder.set_pubkey(&TEST_ROOT_CA_KEY).unwrap();
        cert_builder.set_not_before(&Asn1Time::days_from_now(0).unwrap()).unwrap();
        cert_builder.set_not_after(&Asn1Time::days_from_now(365 * 10).unwrap()).unwrap();

        let basic_constraints = openssl::x509::extension::BasicConstraints::new().critical().ca().build().unwrap();
        cert_builder.append_extension(basic_constraints).unwrap();

        cert_builder.sign(&TEST_ROOT_CA_KEY, MessageDigest::sha256()).unwrap();

        cert_builder.build()
    };

    pub static ref TEST_ROOT_CA_PEM : Vec<u8> = {
        let pem = TEST_ROOT_CA_CERT.to_pem().unwrap();
        pem
    };

}

#[cfg(not(feature = "test-utils"))]
static AWS_ROOT_CA_PEM: &[u8] = b"-----BEGIN CERTIFICATE-----
MIICETCCAZagAwIBAgIRAPkxdWgbkK/hHUbMtOTn+FYwCgYIKoZIzj0EAwMwSTEL
MAkGA1UEBhMCVVMxDzANBgNVBAoMBkFtYXpvbjEMMAoGA1UECwwDQVdTMRswGQYD
VQQDDBJhd3Mubml0cm8tZW5jbGF2ZXMwHhcNMTkxMDI4MTMyODA1WhcNNDkxMDI4
MTQyODA1WjBJMQswCQYDVQQGEwJVUzEPMA0GA1UECgwGQW1hem9uMQwwCgYDVQQL
DANBV1MxGzAZBgNVBAMMEmF3cy5uaXRyby1lbmNsYXZlczB2MBAGByqGSM49AgEG
BSuBBAAiA2IABPwCVOumCMHzaHDimtqQvkY4MpJzbolL//Zy2YlES1BR5TSksfbb
48C8WBoyt7F2Bw7eEtaaP+ohG2bnUs990d0JX28TcPQXCEPZ3BABIeTPYwEoCWZE
h8l5YoQwTcU/9KNCMEAwDwYDVR0TAQH/BAUwAwEB/zAdBgNVHQ4EFgQUkCW1DdkF
R+eWw5b6cp3PmanfS5YwDgYDVR0PAQH/BAQDAgGGMAoGCCqGSM49BAMDA2kAMGYC
MQCjfy+Rocm9Xue4YnwWmNJVA44fA0P5W2OpYow9OYCVRaEevL8uO1XYru5xtMPW
rfMCMQCi85sWBbJwKKXdS6BptQFuZbT73o/gBh1qUxl/nNr12UO8Yfwr6wPLb+6N
IwLz3/Y=
-----END CERTIFICATE-----";

impl NitroAttestationDocument {
    // TODO: consider time validation.
    fn verify_cert_chain(leaf_cert: &X509, ca_certs: &[X509], root_cert: &X509) -> Result<()> {
        use openssl::stack::Stack;
        use openssl::x509::store::X509StoreBuilder;
        use openssl::x509::X509StoreContext;
        // Create a new store and add the root cert
        let mut store = X509StoreBuilder::new()?;
        store.add_cert(root_cert.clone())?;
        let store = store.build();
        // Create a stack for the intermediate certs
        let mut stack = Stack::new()?;
        for cert in ca_certs {
            stack.push(cert.clone())?;
        }
        // Create store context and verify
        let mut ctx = X509StoreContext::new()?;
        let verifier = |cref: &mut openssl::x509::X509StoreContextRef| {
            let verify_result = cref.verify_cert()?;
            if !verify_result {
                tracing::error!(
                    "certificate error: '{}' depth {}",
                    cref.error(),
                    cref.error_depth()
                );
            }
            Ok(verify_result)
        };
        let ok = ctx.init(&store, leaf_cert, &stack, verifier)?;
        if !ok {
            bail!("certificate chain verification failed")
        }
        Ok(())
    }

    // TODO: What about the digest field?
    fn verify_nitro_attestation(cose: &CoseSign1) -> Result<Self> {
        use aws_nitro_enclaves_cose::crypto::Openssl;
        // Get payload without verification to access the cert chain
        let payload = cose
            .get_payload::<Openssl>(None)
            .map_err(|e| anyhow!("CoseSign1::get_payload: {}", e))?;
        let attestation: NitroAttestationDocument = serde_cbor::from_slice(&payload)?;
        #[cfg(not(feature = "test-utils"))]
        let root_cert_pem = AWS_ROOT_CA_PEM;
        // TODO: remove this once not needed!
        #[cfg(feature = "test-utils")]
        let root_cert_pem = &*TEST_ROOT_CA_PEM;
        // Parse root cert
        let root_cert = X509::from_pem(root_cert_pem)?;
        // Parse leaf cert and bundle
        let leaf_cert = X509::from_der(&attestation.certificate)?;
        let ca_certs: Vec<X509> = attestation
            .cabundle
            .iter()
            .map(|cert_der| X509::from_der(cert_der))
            .collect::<Result<_, _>>()?;
        // Verify cert chain
        Self::verify_cert_chain(&leaf_cert, &ca_certs, &root_cert)?;
        // Get signing key from leaf cert
        let signing_key = leaf_cert.public_key()?;
        // Now verify the COSE signature
        let ok = cose
            .verify_signature::<Openssl>(&signing_key)
            .map_err(|e| anyhow!("CoseSign1::verify_signature: {}", e))?;
        if !ok {
            bail!("signature does not verify");
        }
        Ok(attestation)
    }

    pub fn from_cose(cose_document: &[u8]) -> Result<Self> {
        let cose = CoseSign1::from_bytes(cose_document)
            .map_err(|e| anyhow!("CoseSign1::from_bytes: {}", e))?;
        Self::verify_nitro_attestation(&cose)
    }

    pub fn verify(
        &self,
        expected_pcrs: Option<&std::collections::HashMap<u8, ByteBuf>>,
        expected_public_key: Option<&ByteBuf>,
        expected_user_data: Option<&ByteBuf>,
        expected_nonce: Option<&ByteBuf>,
    ) -> Result<()> {
        if let Some(expected) = expected_pcrs {
            for (&pcr_idx, expected_value) in expected {
                match self.pcrs.get(&pcr_idx) {
                    Some(actual_value) if actual_value == expected_value => {
                        tracing::debug!("PCR{} ok", pcr_idx);
                    }
                    _ => bail!("PCR{} mismatch or not found", pcr_idx),
                }
            }
        }
        if let Some(expected) = expected_public_key {
            match self.public_key.as_ref() {
                Some(actual) if actual == expected => {
                    tracing::debug!("public_key ok");
                }
                _ => bail!("public key mismatch"),
            }
        }
        if let Some(expected) = expected_user_data {
            match self.user_data.as_ref() {
                Some(actual) if actual == expected => {
                    tracing::debug!("user_data ok");
                }
                _ => bail!("user data mismatch"),
            }
        }
        if let Some(expected) = expected_nonce {
            match self.nonce.as_ref() {
                Some(actual) if actual == expected => {
                    tracing::debug!("nonce ok");
                }
                _ => bail!("nonce mismatch"),
            }
        }
        Ok(())
    }
}

impl NitroAttestationDocument {
    /// Generate a Cose1 envelope with an AWS-like attestation document
    /// with the specified components. The document is signed by the test key.
    #[cfg(feature = "test-utils")]
    pub fn cose_create(
        pcrs: std::collections::HashMap<u8, ByteBuf>,
        public_key: Option<ByteBuf>,
        user_data: Option<ByteBuf>,
        nonce: Option<ByteBuf>,
    ) -> Result<Vec<u8>> {
        // Generate leaf certificate signed by the test root CA
        let ec_group = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::X9_62_PRIME256V1)?;
        let ec_key = openssl::ec::EcKey::generate(&ec_group)?;
        let leaf_key = PKey::from_ec_key(ec_key)?;
        //let leaf_key = PKey::generate_ed25519().unwrap();

        let mut x509_name = X509NameBuilder::new()?;
        x509_name.append_entry_by_text("C", "US")?;
        x509_name.append_entry_by_text("O", "Test Organization")?;
        x509_name.append_entry_by_text("CN", "Test Leaf Certificate")?;
        let x509_name = x509_name.build();

        let mut cert_builder = X509::builder()?;
        cert_builder.set_version(2)?;
        cert_builder.set_subject_name(&x509_name)?;
        cert_builder.set_issuer_name(TEST_ROOT_CA_CERT.subject_name())?;
        cert_builder.set_pubkey(&leaf_key)?;

        use anyhow::Context;
        cert_builder.set_not_before(Asn1Time::days_from_now(0).context("asn1")?.as_ref())?;
        cert_builder.set_not_after(Asn1Time::days_from_now(365).context("asn1")?.as_ref())?;
        cert_builder.sign(&TEST_ROOT_CA_KEY, MessageDigest::sha256())?;
        let cert = cert_builder.build();

        let doc = Self {
            module_id: "test-module".to_string(),
            digest: "test-digest".to_string(),
            timestamp: 1234567890,
            pcrs,
            certificate: ByteBuf::from(cert.to_der()?),
            cabundle: vec![ByteBuf::from(TEST_ROOT_CA_CERT.to_der()?)],
            public_key,
            user_data,
            nonce,
        };

        let payload = serde_cbor::to_vec(&doc)?;

        // Create a signing key adapter
        //let signing_key = OpenSslPrivateKey::new(&leaf_key);

        // Create CoseSign1 document with empty unprotected headers
        let cose = CoseSign1::new::<aws_nitro_enclaves_cose::crypto::Openssl>(
            &payload,
            &aws_nitro_enclaves_cose::header_map::HeaderMap::new(),
            &leaf_key,
        )
        .map_err(|x| anyhow!("CoseSign1::new: {}", x))?;

        let cose_bytes = cose.as_bytes(true).map_err(|x| anyhow!("CoseSign1::as_bytes: {}", x))?;

        Ok(cose_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_bytes::ByteBuf;
    use std::collections::HashMap;

    #[test]
    fn test_cose_create_and_verify() {
        tracing_subscriber::fmt()
            .with_thread_ids(true)
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            // Shows TRACE, DEBUG, INFO, WARN, ERROR
            .with_max_level(tracing::Level::TRACE)
            .init();

        // Prepare test data
        let mut pcrs = HashMap::new();
        pcrs.insert(0, ByteBuf::from(vec![0; 48]));

        let public_key = Some(ByteBuf::from(b"test-public-key"));
        let user_data = Some(ByteBuf::from(b"test-user-data"));
        let nonce = Some(ByteBuf::from(b"test-nonce"));

        // Create COSE document
        let cose_doc = NitroAttestationDocument::cose_create(
            pcrs.clone(),
            public_key.clone(),
            user_data.clone(),
            nonce.clone(),
        )
        .expect("Failed to create COSE document");

        // Verify the document
        let attestation =
            NitroAttestationDocument::from_cose(&cose_doc).expect("Failed to parse COSE document");

        // Verify contents
        assert_eq!(attestation.module_id, "test-module");
        assert_eq!(attestation.digest, "test-digest");

        // Verify PCRs
        assert_eq!(attestation.pcrs, pcrs);

        // Verify optional fields
        assert_eq!(attestation.public_key, public_key);
        assert_eq!(attestation.user_data, user_data);
        assert_eq!(attestation.nonce, nonce);

        attestation
            .verify(Some(&pcrs), public_key.as_ref(), user_data.as_ref(), nonce.as_ref())
            .expect("Verification should succeed");

        // Test verify method with mismatched values
        let mut wrong_pcrs = pcrs.clone();
        wrong_pcrs.insert(1, ByteBuf::from(vec![1; 48]));

        assert!(
            attestation
                .verify(Some(&wrong_pcrs), public_key.as_ref(), user_data.as_ref(), nonce.as_ref())
                .is_err(),
            "Verification should fail with wrong PCRs"
        );
    }
}
