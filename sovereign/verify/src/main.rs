use aws_nitro_enclaves_cose::{crypto::Openssl, CoseSign1};
use k256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde::Deserialize;
use serde_bytes::ByteBuf;
use std::collections::BTreeMap;

use clap::Parser;
use reqwest::{self};

mod cert;

// TODO: update this with changes to enclave!!!

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Base URL of the enclave proxy")]
    url: String,
}

#[derive(Debug, Deserialize)]
pub struct NitroAttestationDocument {
    pub module_id: String,
    pub digest: String,
    pub timestamp: u64,
    pcrs: std::collections::HashMap<u8, ByteBuf>,
    certificate: ByteBuf,
    cabundle: Vec<ByteBuf>,
    public_key: Option<ByteBuf>,
    user_data: Option<ByteBuf>,
    nonce: Option<ByteBuf>,
}

fn verify_attestation(
    root_cert: &[u8],
    cose_document: &[u8],
    expected_pcrs: Option<&BTreeMap<u8, Vec<u8>>>,
    expected_public_key: Option<&[u8]>,
    expected_user_data: Option<&[u8]>,
    expected_nonce: Option<&[u8]>,
) -> Result<NitroAttestationDocument, Box<dyn std::error::Error>> {
    tracing::debug!("Cose from bytes...");
    let cose_sign1 = CoseSign1::from_bytes(cose_document)?;
    tracing::debug!("Cose get payload...");
    let payload: Vec<u8> = cose_sign1.get_payload::<Openssl>(None)?;
    tracing::debug!("Serde from slice...");
    let doc: NitroAttestationDocument = serde_cbor::from_slice(&payload)?;
    tracing::debug!("Attestation document: {:#?}", doc);
    if let Some(expected) = expected_pcrs {
        for (&pcr_idx, expected_value) in expected {
            match doc.pcrs.get(&pcr_idx) {
                Some(actual_value) if actual_value == expected_value => {
                    tracing::debug!("PCR{} ok", pcr_idx);
                }
                _ => return Err(format!("PCR{} mismatch or not found", pcr_idx).into()),
            }
        }
    }
    if let Some(expected) = expected_public_key {
        match doc.public_key.as_ref() {
            Some(actual) => {
                if actual.as_slice() == expected {
                    tracing::debug!("public_key ok");
                } else {
                    return Err(format!(
                        "public key mismatch: expected {:#?}, actual {:#?}",
                        expected, actual
                    )
                    .into());
                }
            }
            _ => return Err(format!("missing public key in attestation document").into()),
        }
    }
    if let Some(expected) = expected_user_data {
        match doc.user_data.as_ref() {
            Some(actual) if actual.as_slice() == expected => {
                tracing::debug!("user_data ok");
            }
            _ => return Err("User data mismatch".into()),
        }
    }
    if let Some(expected) = expected_nonce {
        match doc.nonce.as_ref() {
            Some(actual) if actual.as_slice() == expected => {
                tracing::debug!("nonce ok");
            }
            _ => return Err("User data mismatch".into()),
        }
    }
    cert::verify_certificate(root_cert, &doc.certificate, &doc.cabundle)?;
    Ok(doc)
}

async fn verify_main(args_url: &str, root_cert: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let base_url = format!("http://{}", args_url);

    let client = reqwest::Client::new();

    // 1. Get Attestation Document
    let attestation_doc = client
        .get(&format!("{}/attestation?encoding=binary", base_url))
        .send()
        .await?
        .bytes()
        .await?;

    tracing::info!("Attestation Document ({} bytes)", attestation_doc.len());

    // 2. Get Public Keys
    let pubkey1 = client
        .get(&format!("{}/public_key", base_url))
        .header("x-public-key", "1")
        .send()
        .await?
        .bytes()
        .await?;

    let pubkey2 = client
        .get(&format!("{}/public_key", base_url))
        .header("x-public-key", "2")
        .send()
        .await?
        .bytes()
        .await?;

    tracing::info!("Pubkeys: 1 {} bytes, 2 {} bytes", pubkey1.len(), pubkey2.len());

    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&pubkey1);
    hasher.update(&pubkey2);
    let _expected_public_key = hasher.finalize(); // this is a 32-byte array

    verify_attestation(root_cert, attestation_doc.as_ref(), None, None, None, None)?;

    // 3. Signing Test
    // Prepare test vector [0, 1, ..., 31]
    let test_vector: Vec<u8> = (0..32).collect();

    // Sign with key 1
    let signature1 = client
        .post(&format!("{}/sign", base_url))
        .header("x-ecdsa-signing-key", "1")
        .body(test_vector.clone())
        .send()
        .await?
        .bytes()
        .await?;

    // Sign with key 2
    let signature2 = client
        .post(&format!("{}/sign", base_url))
        .header("x-ecdsa-signing-key", "2")
        .body(test_vector.clone())
        .send()
        .await?
        .bytes()
        .await?;

    // 4. Verify Signatures
    let verifying_key1 = VerifyingKey::from_sec1_bytes(&pubkey1)?;
    let verifying_key2 = VerifyingKey::from_sec1_bytes(&pubkey2)?;

    // Verify signatures
    let signature1_obj = Signature::from_slice(&signature1)?;
    let signature2_obj = Signature::from_slice(&signature2)?;

    verifying_key1.verify(&test_vector, &signature1_obj)?;
    verifying_key2.verify(&test_vector, &signature2_obj)?;

    println!("Signatures verified successfully!");

    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_thread_ids(true)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        // Shows TRACE, DEBUG, INFO, WARN, ERROR
        .with_max_level(tracing::Level::TRACE)
        .init();

    let args = Args::parse();

    // TODO: Available as a file from xxx
    const AWS_ROOT_CA_PEM: &[u8] = b"-----BEGIN CERTIFICATE-----
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

    let pems = pem::parse_many(AWS_ROOT_CA_PEM).unwrap();
    assert_eq!(pems.len(), 1);
    let pem = &pems[0];

    if let Err(e) = verify_main(&args.url, pem.contents()).await {
        tracing::error!("Error: {}", e);
        std::process::exit(1);
    }
}
