use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, UnixTime};
use rustls::server::ParsedCertificate;
use rustls::{client::verify_server_cert_signed_by_trust_anchor, RootCertStore};
use rustls_pki_types::SignatureVerificationAlgorithm; // Add this import
use serde_bytes::ByteBuf;
use std::error::Error as StdError;

pub fn verify_certificate(
    root_ca: &[u8],
    cert_bytes: &[u8],
    ca_bundle: &Vec<ByteBuf>,
) -> Result<(), Box<dyn StdError>> {
    // Create root store
    let mut root_store = RootCertStore::empty();
    root_store.add(CertificateDer::from(root_ca.to_vec()))?;

    // Convert cert to ParsedCertificate
    let cert_der = CertificateDer::from(cert_bytes.to_vec());
    let cert = ParsedCertificate::try_from(&cert_der)?;

    // Convert intermediates to CertificateDer
    let intermediates: Vec<CertificateDer> =
        ca_bundle.iter().map(|cert| CertificateDer::from(cert.to_vec())).collect();

    // Current time for certificate validation
    let now = UnixTime::now();

    let provider = default_provider();
    let supported_algs: &[&dyn SignatureVerificationAlgorithm] =
        provider.signature_verification_algorithms.all;

    // Verify certificate
    match verify_server_cert_signed_by_trust_anchor(
        &cert,
        &root_store,
        &intermediates,
        now,
        supported_algs,
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
