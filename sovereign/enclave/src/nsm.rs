//! Implementation of the security module trait for AWS NSM.

use anyhow::{anyhow, bail, Result};
use serde_bytes::ByteBuf;
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};

use crate::secmod::{AttestationDocument, Secmod};

pub struct Nsm;

/// See [AWS Attestation](https://docs.aws.amazon.com/enclaves/latest/user/set-up-attestation.html).
impl AttestationDocument for nsm_attestation::NitroAttestationDocument {
    fn code_measurement(&self) -> String {
        let pcrs = &self.pcrs;
        // Get PCR values 0,1,2 which contain code measurements
        let pcr0 = pcrs.get(&0).map(hex::encode).unwrap_or_default();
        let pcr1 = pcrs.get(&1).map(hex::encode).unwrap_or_default();
        let pcr2 = pcrs.get(&2).map(hex::encode).unwrap_or_default();
        // Construct code measurement message
        format!("AWS-CODE:{}:{}:{}", pcr0, pcr1, pcr2)
    }

    fn instance_measurement(&self) -> String {
        let pcrs = &self.pcrs;
        // PCR-4 contains the instance measurement.
        let pcr4 = pcrs.get(&4).map(hex::encode).unwrap_or_default();
        // Construct instance measurement message
        format!("AWS-INSTANCE:{}", pcr4)
    }

    fn nonce(&self) -> Option<&ByteBuf> {
        self.nonce.as_ref()
    }
    fn public_key(&self) -> Option<&ByteBuf> {
        self.public_key.as_ref()
    }
    fn user_data(&self) -> Option<&ByteBuf> {
        self.user_data.as_ref()
    }
}

fn extend_pcr(nsm_fd: i32, index: u16, data: Vec<u8>) -> Result<()> {
    let describe_request = nsm_io::Request::DescribePCR { index };
    match nsm_driver::nsm_process_request(nsm_fd, describe_request) {
        nsm_io::Response::DescribePCR { lock, data: old_data } => {
            if lock {
                bail!("PCR#{} is locked", index)
            }
            if old_data.len() != 48 {
                bail!("PCR#{} wrong length {} (expected 48)", index, old_data.len())
            }
            if old_data != [0; 48] {
                bail!("PCR#{} already in use (non-zero)", index)
            }
        }
        _ => bail!("cannot describe PCR#{}", index),
    }
    // Extending a PCR replaces its `old_hash` with `new_hash`
    // where `new_hash=SHA384(old_hash | new_data)` and `|` is concatenation.
    // Unused PCRs start of with 48 zero bytes.
    let extend_request = nsm_io::Request::ExtendPCR { index, data: data.clone() };
    match nsm_driver::nsm_process_request(nsm_fd, extend_request) {
        nsm_io::Response::ExtendPCR { data: new_hash } => {
            use sha2::Digest;
            let mut hasher = sha2::Sha384::new();
            hasher.update([0; 48]);
            hasher.update(data);
            let hash = hasher.finalize().to_vec();
            if hash != new_hash {
                bail!("extension incorrect for PCR#{}", index)
            }
        }
        _ => bail!("cannot extend PCR#{}", index),
    }
    let lock_request = nsm_io::Request::LockPCR { index };
    match nsm_driver::nsm_process_request(nsm_fd, lock_request) {
        nsm_io::Response::LockPCR => {}
        _ => bail!("cannot lock PCR#{}", index),
    }
    Ok(())
}

impl Secmod for Nsm {
    type Att = nsm_attestation::NitroAttestationDocument;
    type Listener = VsockListener;
    type Stream = VsockStream;
    type Attestor = i32;

    fn listen(
        port: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Listener>> + Send>> {
        Box::pin(async move {
            let addr = VsockAddr::new(tokio_vsock::VMADDR_CID_ANY, port);
            let listener = tokio_vsock::VsockListener::bind(addr)?;
            Ok(listener)
        })
    }

    fn connect(
        port: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Stream>> + Send>> {
        Box::pin(async move {
            // TODO: remove magic port number
            let addr = VsockAddr::new(3, port);
            let stream = VsockStream::connect(addr)
                .await
                .map_err(|x| anyhow!("failed to connect to VSOCK {}: {}", addr, x.to_string()))?;
            Ok(stream)
        })
    }

    fn accept(
        listener: &Self::Listener,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Stream>> + Send + '_>>
    {
        Box::pin(async move {
            let (stream, _addr) = listener.accept().await?;
            Ok(stream)
        })
    }

    /// Here `code` is expected to have the format `{PCR-0}:{PCR-1}:{PCR-2}`.
    /// This code returns this string prefixed with "AWS-CODE:".
    fn measure_code(code: String) -> String {
        format!("AWS-CODE:{}", code)
    }

    /// The AWS  Nitro code measurement is zero for debug enclaves.
    /// Each PCR is 48 zero bytes or 96 digit zero characters in hex.
    fn measure_debug_code() -> String {
        let x = "0".repeat(48 * 2);
        Self::measure_code(format!("{}:{}:{}", x, x, x))
    }

    /// In AWS Nitro, the instance measurement in PCR-4 is `pcr4=SHA384([0; 48] | instance)`,
    /// where `instance` is the the host's instance ID (e.g., "i-1234567890abcdef0").
    ///
    /// This function returns the string "AWS-INSTANCE:{pcr4}".
    /// This string can then be compared with the actual value of
    /// `doc.instance_measurement()` for a valid attestation document `doc`.
    fn measure_instance(instance: String) -> String {
        use sha2::Digest;
        let mut hasher = sha2::Sha384::new();
        hasher.update([0; 48]);
        hasher.update(instance.as_bytes());
        let hash = hasher.finalize().to_vec();
        let hex_pcr4 = hex::encode(hash);
        format!("AWS-INSTANCE:{}", hex_pcr4)
    }

    /// For AWS Nitro, this calls `nsm_driver::nsm_init` to get a file descriptor.
    fn init_attestor() -> Result<Self::Attestor> {
        tracing::info!("initializing NSM...");
        let nsm_fd = nsm_driver::nsm_init();
        if nsm_fd < 0 {
            bail!("failed to initialize NSM")
        }
        Ok(nsm_fd)
    }

    fn new_attestation(
        attestor: &Self::Attestor,
        nonce: Option<ByteBuf>,
        public_key: Option<ByteBuf>,
        user_data: Option<ByteBuf>,
    ) -> Result<Vec<u8>> {
        let request = nsm_io::Request::Attestation { public_key, user_data, nonce };
        match nsm_driver::nsm_process_request(*attestor, request) {
            nsm_io::Response::Attestation { document } => Ok(document),
            _ => bail!("cannot create attestation"),
        }
    }

    fn parse(doc: &[u8]) -> Result<Self::Att> {
        nsm_attestation::NitroAttestationDocument::from_cose(doc)
    }

    fn measure_enclave(attestor: &Self::Attestor, measurements: Vec<Vec<u8>>) -> Result<()> {
        if measurements.len() > 16 {
            bail!("at most 16 measurements supported, was {}", measurements.len());
        }
        tracing::info!("extending PCRs with config and public keys");
        for (index, data) in measurements.into_iter().enumerate() {
            extend_pcr(*attestor, (index + 16) as u16, data)?;
        }
        Ok(())
    }
}
