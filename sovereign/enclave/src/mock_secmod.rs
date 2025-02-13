//! Mock security module. Not secure, but good for testing.

use std::collections::HashMap;

use anyhow::Result;
use serde_bytes::ByteBuf;
use tokio::net::{TcpListener, TcpStream};

use crate::secmod::{AttestationDocument, Secmod};

pub struct MockSecmod;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MockAttestationDocument {
    pub pcrs: std::collections::HashMap<u8, ByteBuf>,
    pub public_key: Option<ByteBuf>,
    pub user_data: Option<ByteBuf>,
    pub nonce: Option<ByteBuf>,
}

impl AttestationDocument for MockAttestationDocument {
    fn code_measurement(&self) -> String {
        let pcrs = &self.pcrs;
        let pcr0 = pcrs.get(&0).map(hex::encode).unwrap_or_default();
        let pcr1 = pcrs.get(&1).map(hex::encode).unwrap_or_default();
        let pcr2 = pcrs.get(&2).map(hex::encode).unwrap_or_default();
        // Construct code measurement message
        format!("MOCK-CODE:{}:{}:{}", pcr0, pcr1, pcr2)
    }

    fn instance_measurement(&self) -> String {
        let pcrs = &self.pcrs;
        // PCR-4 contains the instance measurement.
        let pcr4 = pcrs.get(&4).map(hex::encode).unwrap_or_default();
        // Construct instance measurement message
        format!("MOCK-INSTANCE:{}", pcr4)
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

#[derive(Debug, Clone, Copy)]
pub enum MockAttestor {
    #[cfg(test)]
    Debug,
    ProdLike,
}

#[cfg(test)]
impl MockSecmod {
    /// Produce an attestor that produces "debug" attestations.
    pub fn init_debug_attestor() -> <MockSecmod as Secmod>::Attestor {
        MockAttestor::Debug
    }
}

impl Secmod for MockSecmod {
    type Att = MockAttestationDocument;
    type Listener = TcpListener;
    type Stream = TcpStream;
    type Attestor = MockAttestor;

    fn listen(
        port: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Listener>> + Send>> {
        Box::pin(async move {
            let addr = format!("localhost:{}", port);
            tracing::debug!("mock TCP listen {}", addr);
            let listener = TcpListener::bind(addr).await?;
            Ok(listener)
        })
    }

    fn connect(
        port: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Stream>> + Send>> {
        Box::pin(async move {
            let addr = format!("localhost:{}", port);
            tracing::debug!("mock TCP connect {}", addr);
            let stream = TcpStream::connect(format!("localhost:{}", port)).await?;
            Ok(stream)
        })
    }

    fn accept(
        listener: &Self::Listener,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Stream>> + Send + '_>>
    {
        Box::pin(async move {
            // tracing::debug!("mock TCP accept");
            let (stream, addr) = listener.accept().await?;
            tracing::debug!("mock TCP accepted on {} from {}", listener.local_addr()?, addr);
            Ok(stream)
        })
    }

    fn measure_code(code: String) -> String {
        format!("MOCK-CODE:{}", code)
    }

    fn measure_debug_code() -> String {
        format!("MOCK-CODE:00:00:00")
    }

    fn measure_instance(instance: String) -> String {
        format!("MOCK-INSTANCE:{}", instance)
    }

    fn init_attestor() -> Result<Self::Attestor> {
        Ok(MockAttestor::ProdLike)
    }

    fn new_attestation(
        attestor: &Self::Attestor,
        nonce: Option<ByteBuf>,
        public_key: Option<ByteBuf>,
        user_data: Option<ByteBuf>,
    ) -> Result<Vec<u8>> {
        let pcr = match attestor {
            #[cfg(test)]
            MockAttestor::Debug => ByteBuf::from([0u8; 1]),
            MockAttestor::ProdLike => ByteBuf::from([0xffu8; 1]),
        };
        let v = serde_json::to_vec(&nsm_attestation::NitroAttestationDocument {
            nonce,
            public_key,
            user_data,
            module_id: "mock module ID".to_string(),
            digest: "mock digest".to_string(),
            pcrs: HashMap::from([
                (0, pcr.clone()),
                (1, pcr.clone()),
                (2, pcr.clone()),
                (4, ByteBuf::from([0xabu8; 1])),
            ]),
            timestamp: 1066,
            certificate: ByteBuf::new(),
            cabundle: Vec::new(),
        })?;
        Ok(v)
    }

    fn parse(doc: &[u8]) -> Result<Self::Att> {
        let att = serde_json::from_slice(doc)?;
        Ok(att)
    }

    fn measure_enclave(attestor: &Self::Attestor, data: Vec<Vec<u8>>) -> Result<()> {
        tracing::info!("measure_enclave({:?}, {} items)", attestor, data.len());
        Ok(())
    }
}
