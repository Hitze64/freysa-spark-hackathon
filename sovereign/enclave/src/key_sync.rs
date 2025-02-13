//! This module implements the key-sync protocol.

use crate::{AttestationDocument, Secmod};
use anyhow::{anyhow, bail, Result};
use elliptic_curve::rand_core::{self, RngCore};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing;

async fn authorize_measurements<SM: Secmod + 'static>(
    attestor: &SM::Attestor,
    gov: &crate::config::Governance,
    att: &SM::Att,
) -> Result<()> {
    use crate::config::Governance;
    match gov {
        Governance::TestingOnly => {
            if att.code_measurement() != SM::measure_debug_code() {
                bail!(
                    "remote attestation not debug; was {} expected {}",
                    att.code_measurement(),
                    SM::measure_debug_code()
                )
            }
            let self_att_bytes: Vec<u8> = SM::new_attestation(attestor, None, None, None)?;
            // We parse our own attestation document to get our PCR values.
            let self_att = SM::parse(&self_att_bytes)?;
            if self_att.code_measurement() != SM::measure_debug_code() {
                bail!(
                    "self attestation not debug; was {} expected {}",
                    self_att.code_measurement(),
                    SM::measure_debug_code()
                )
            }
            tracing::warn!("authorizing measurements in debug mode");
            Ok(())
        }
        Governance::Safe(config) => {
            crate::safe::safe_authorize_message::<SM>(config, &att.code_measurement()).await?;
            // TODO: Should also add instance measurement like so:
            //crate::safe::safe_authorize_message::<SM>(config, &att.instance_measurement()).await?;
            Ok(())
        }
    }
}

// First message: from leader to follower.
#[derive(Serialize, Deserialize)]
struct RemoteConfigMessage1 {
    leader_nonce: [u8; 32],
}

// Second message: from follower to leader.
#[derive(Serialize, Deserialize)]
struct RemoteConfigMessage2 {
    // Should contain
    // nonce = leader_nonce,
    // public_key = follower public key
    // user_data = follower_nonce
    attestation_doc: Vec<u8>,
}

// Third message: from leader to follower.
#[derive(Serialize, Deserialize)]
struct RemoteConfigMessage3 {
    // Should contain
    // nonce = follower_nonce,
    // user_data = hash(encrypted_message)
    attestation_doc: Vec<u8>,
    // RemoteConfigMessage3Contents encrypted with follower public key
    encrypted_message: Vec<u8>,
}

pub async fn read_message<R>(stream: &mut R) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    // Read message length (4 bytes)
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    // 64Mib
    const MAX_LEN: usize = 1 << 26;
    if len > MAX_LEN {
        bail!("refuse to read message larger than {} bytes (was {})", MAX_LEN, len)
    }
    let mut buffer = vec![0; len];
    // Read actual message
    stream.read_exact(&mut buffer).await?;
    Ok(buffer.to_vec())
}

async fn write_message<W>(stream: &mut W, msg: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let len_bytes = (msg.len() as u32).to_be_bytes();
    stream.write_all(&len_bytes).await?;
    stream.write_all(msg).await?;
    Ok(())
}

pub async fn serve_follower_key_sync<SM: Secmod + 'static, T>(
    attestor: &SM::Attestor,
    governance: &crate::config::Governance,
    stream: &mut T,
) -> Result<Vec<u8>>
where
    T: AsyncRead,
    T: AsyncWrite,
    T: Unpin,
{
    // Read message
    let message1_bytes = read_message(stream).await?;
    let message1: RemoteConfigMessage1 = serde_json::from_slice(&message1_bytes)?;
    let leader_nonce: [u8; 32] = message1.leader_nonce;
    tracing::info!("follower: received remote configuration request");
    // Generate follower components
    let sec = k256::SecretKey::random(&mut rand_core::OsRng);
    let pubk = sec.public_key();
    let follower_nonce = random_nonce()?;
    // Generate attestation document with leader's nonce and our public key
    let follower_att: Vec<u8> = SM::new_attestation(
        attestor,
        Some(ByteBuf::from(leader_nonce)),
        Some(ByteBuf::from(pubk.to_sec1_bytes())),
        Some(ByteBuf::from(follower_nonce)),
    )?;
    // Send response with attestation doc
    let message2 = RemoteConfigMessage2 { attestation_doc: follower_att };
    let message2_bytes = serde_json::to_vec(&message2)?;
    tracing::trace!("follower: write message 2 / {} bytes", message2_bytes.len());
    write_message(stream, &message2_bytes).await?;
    // Wait for leader's response
    tracing::info!("follower: waiting for attestation and encrypted message");
    let message3_bytes = read_message(stream).await?;
    tracing::trace!("follower: read message 3 / {} bytes", message3_bytes.len());
    let message3: RemoteConfigMessage3 = serde_json::from_slice(&message3_bytes)?;
    let leader_att = SM::parse(&message3.attestation_doc)?;
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&message3.encrypted_message);
    let enc_sha = hasher.finalize();
    use crate::secmod::AttestationDocumentExt;
    leader_att.verify(
        Some(&ByteBuf::from(&follower_nonce)),
        None,
        Some(&enc_sha.to_vec().into()),
    )?;
    authorize_measurements::<SM>(&attestor, governance, &leader_att).await?;
    // Decrypt the configuration using our secret key
    let message_bytes = ecies::decrypt(&sec.to_bytes().as_slice(), &message3.encrypted_message)
        .map_err(|x| anyhow!("decrypt {}", x))?;
    tracing::info!("key-sync successful (follower)");
    Ok(message_bytes)
}

pub async fn serve_leader_key_sync<SM: Secmod + 'static, T>(
    attestor: &SM::Attestor,
    governance: &crate::config::Governance,
    key_material: &[u8],
    stream: &mut T,
) -> Result<()>
where
    T: AsyncRead,
    T: AsyncWrite,
    T: Unpin,
{
    let leader_nonce = random_nonce()?;
    let message1 = RemoteConfigMessage1 { leader_nonce };
    let message1_bytes = serde_json::to_vec(&message1)?;
    tracing::trace!("leader: write message 1 / {} bytes", message1_bytes.len());
    write_message(stream, &message1_bytes).await?;
    let message2_bytes = read_message(stream).await?;
    tracing::trace!("leader: read message 2 / {} bytes", message2_bytes.len());
    let message2: RemoteConfigMessage2 = serde_json::from_slice(&message2_bytes)?;
    let follower_att = SM::parse(&message2.attestation_doc)?;
    use crate::secmod::AttestationDocumentExt;
    follower_att.verify(Some(&ByteBuf::from(&leader_nonce)), None, None)?;
    let default_buf = ByteBuf::new();
    let follower_nonce = follower_att.user_data().unwrap_or(&default_buf);
    // Ensure that the follower's PCRs are authorized.
    authorize_measurements::<SM>(&attestor, governance, &follower_att).await?;
    let ss = key_material;
    let pubk = follower_att.public_key().unwrap_or(&default_buf);
    if pubk.len() < 32 {
        bail!("follower public key must be at least 32 bytes")
    }
    let enc_ss = ecies::encrypt(&pubk, ss).map_err(|x| anyhow!("encrypt {}", x))?;
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&enc_ss);
    let enc_sha = hasher.finalize();
    // Now we generate an attestation document using the follower_nonce and enc_sha.
    let leader_att: Vec<u8> = SM::new_attestation(
        &attestor,
        Some(follower_nonce.clone()),
        None,
        Some(enc_sha.to_vec().into()),
    )?;
    let message3 = RemoteConfigMessage3 { attestation_doc: leader_att, encrypted_message: enc_ss };
    let message3_bytes = serde_json::to_vec(&message3)?;
    tracing::trace!("leader: write message 3 / {} bytes", message3_bytes.len());
    write_message(stream, &message3_bytes).await?;
    Ok(())
}

fn random_nonce() -> Result<[u8; 32]> {
    let mut nonce = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut nonce); // Uses system RNG source, not NSM
    Ok(nonce)
}

#[cfg(test)]
#[cfg(feature = "test-utils")]
mod tests {

    use crate::mock_secmod::MockSecmod;

    use crate::config::{SovereignConfig, *};

    use super::*;

    #[tokio::test]
    async fn test_key_sync() -> Result<()> {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            // Shows TRACE, DEBUG, INFO, WARN, ERROR
            .with_max_level(tracing::Level::TRACE)
            .init();

        // Create an in-memory pipe for communication
        let (mut server_stream, mut client_stream) = tokio::io::duplex(1024);

        // Create a mock state
        let secret = vec![0xaau8, 0xbbu8, 0xccu8];
        // Pretend debug mode so authorize measurements using test mode is allowed.
        let attestor = MockSecmod::init_debug_attestor();
        let config =
            SovereignConfig { governance: Governance::TestingOnly, ..SovereignConfig::default() };

        // Spawn the serve_leader_key_sync in a task
        let serve_handle = tokio::spawn({
            let governance = config.governance.clone();
            let secret = secret.clone();
            async move {
                tracing::trace!("starting serve_leader_key_sync");
                let result = serve_leader_key_sync::<MockSecmod, _>(
                    &attestor,
                    &governance,
                    &secret,
                    &mut server_stream,
                )
                .await;
                tracing::trace!("finisehd serve_leader_key_sync");
                result
            }
        });

        // Spawn the serve_follower_key_sync in another task
        let config_handle = tokio::spawn({
            let governance = config.governance.clone();
            async move {
                tracing::trace!("starting serve_follower_key_sync");
                let result = serve_follower_key_sync::<MockSecmod, _>(
                    &attestor,
                    &governance,
                    &mut client_stream,
                )
                .await;
                tracing::trace!("finished serve_follower_key_sync");
                result
            }
        });

        // Wait for both tasks to complete
        let (serve_result, config_result) = tokio::join!(serve_handle, config_handle);
        fn handle_join_result<T>(result: Result<Result<T>, tokio::task::JoinError>) -> Result<T> {
            result.map_err(|e| anyhow!("join error: {}", e))?
        }
        fn combine_results<B>(a: Result<()>, b: Result<B>) -> Result<B> {
            match (a, b) {
                (Ok(()), b) => b,
                (Err(a_err), Err(b_err)) => {
                    Err(anyhow!("multiple errors: {} and {}", a_err, b_err))
                }
                (Err(a_err), Ok(_)) => Err(a_err),
            }
        }
        let follower_secret =
            combine_results(handle_join_result(serve_result), handle_join_result(config_result))?;
        assert!(follower_secret == secret);
        Ok(())
    }
}
