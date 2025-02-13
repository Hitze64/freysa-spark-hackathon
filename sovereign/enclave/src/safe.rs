//! This module implements interaction with a Safe Ethereum smart contract.

use anyhow::{bail, Context, Result};
use hyper::{Method, Request, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tiny_keccak::{Hasher, Keccak};

use crate::config::SafeConfig;

pub async fn safe_authorize_message<SM: crate::secmod::Secmod + 'static>(
    config: &SafeConfig,
    message: &str,
) -> Result<()> {
    let SafeConfig { wallet_address, threshold, http_endpoint_port, http_endpoint, chain_id } =
        config;

    // Check for revocation first
    let revoke_message = format!("REVOKE: {}", message);
    let revoke_hash = safe_hash(*chain_id, &wallet_address, &revoke_message);
    match fetch_safe_message::<SM>(*http_endpoint_port, http_endpoint, &revoke_hash).await? {
        FetchResult::Found(_) => bail!("message has been revoked"),
        FetchResult::NotFound => (), // This is what we want - no revocation exists
    }

    // Now check the actual message
    let message_hash = safe_hash(*chain_id, &wallet_address, message);
    let safe_message =
        match fetch_safe_message::<SM>(*http_endpoint_port, http_endpoint, &message_hash).await? {
            FetchResult::Found(msg) => msg,
            FetchResult::NotFound => bail!("message not found"),
        };

    if safe_message.safe != *wallet_address {
        bail!("safe address mismatch");
    }
    if safe_message.confirmations.len() < *threshold {
        bail!("not enough confirmations");
    }
    tracing::info!("authorizing message using 'safe': {}", message);
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct SafeMessageConfirmation {
    pub owner: String,
    pub signature: String,
    #[serde(rename = "signatureType")]
    pub signature_type: String,
    #[serde(rename = "created")]
    pub created_at: String,
    #[serde(rename = "modified")]
    pub modified_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SafeMessage {
    pub created: String,
    pub modified: String,
    pub safe: String,
    #[serde(rename = "messageHash")]
    pub message_hash: String,
    pub message: String,
    #[serde(rename = "proposedBy")]
    pub proposed_by: String,
    #[serde(rename = "safeAppId")]
    pub safe_app_id: Option<String>,
    pub confirmations: Vec<SafeMessageConfirmation>,
    #[serde(rename = "preparedSignature")]
    pub prepared_signature: String,
    pub origin: String,
}

#[derive(Debug)]
enum FetchResult {
    Found(SafeMessage),
    NotFound,
}

async fn fetch_safe_message<SM: crate::secmod::Secmod + 'static>(
    out_port: u32,
    http_endpoint: &str,
    message_hash: &str,
) -> Result<FetchResult> {
    let url = format!("{}/{}/", http_endpoint, message_hash);
    let uri = url.parse::<hyper::Uri>()?;
    tracing::debug!(
        "fetch safe message from URI: scheme={:?}, authority={:?}, path={:?}",
        uri.scheme(),
        uri.authority(),
        uri.path()
    );
    let origin = format!(
        "{}://{}",
        uri.scheme_str().context("missing scheme")?,
        uri.authority().context("missing authority")?.host()
    );
    let request = Request::builder()
        .method(Method::GET)
        .uri(&uri)
        .header(hyper::header::ACCEPT, "application/json")
        .header(hyper::header::ORIGIN, origin)
        .body(crate::http::full(Vec::new()))?;

    tracing::trace!("using 'safe' request message {:#?}", request);
    let response = crate::http::make_request::<SM>(out_port, request).await?;

    match response.status() {
        StatusCode::OK => {
            let body = crate::http::get_body(response.into_body(), 1 << 20).await?;
            let message = serde_json::from_slice(&body)?;
            tracing::debug!("fetched safe message: {:#?}", message);
            Ok(FetchResult::Found(message))
        }
        StatusCode::NOT_FOUND => Ok(FetchResult::NotFound),
        status => bail!("invalid response status: {}", status),
    }
}

fn safe_hash(chain_id: u64, safe_address: &str, message: &str) -> String {
    let message_hash = inner_hash(message);
    let typed_data = get_typed_data(chain_id, safe_address, &message_hash);
    let encoding = encode_typed_data(typed_data);
    my_keccak(&encoding)
}

fn my_keccak(data: &[u8]) -> String {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];

    hasher.update(data);
    hasher.finalize(&mut output);

    format!("0x{}", hex::encode(output))
}

fn inner_hash(message: &str) -> String {
    let message_bytes = message.as_bytes();
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message_bytes.len());
    let prefixed = [prefix.as_bytes(), message_bytes].concat();
    my_keccak(&prefixed)
}
fn get_typed_data(chain_id: u64, safe_address: &str, message: &str) -> HashMap<String, Value> {
    let mut typed_data = Vec::new();

    typed_data.push((
        "types".to_string(),
        json!({
            "EIP712Domain": [
                {"type": "uint256", "name": "chainId"},
                {"type": "address", "name": "verifyingContract"}
            ],
            "SafeMessage": [
                {
                    "type": "bytes",
                    "name": "message"
                }
            ]
        }),
    ));

    typed_data.push((
        "domain".to_string(),
        json!({
            "verifyingContract": safe_address,
            "chainId": chain_id
        }),
    ));

    typed_data.push((
        "message".to_string(),
        json!({
            "message": message
        }),
    ));

    typed_data.into_iter().collect()
}

fn encode_typed_data(typed_data: HashMap<String, Value>) -> Vec<u8> {
    let domain = typed_data.get("domain").unwrap().as_object().unwrap();
    let types = typed_data.get("types").unwrap().as_object().unwrap();
    let message = typed_data.get("message").unwrap().as_object().unwrap();

    let domain_hash = hash_struct("EIP712Domain", domain, types);
    let message_hash = hash_struct("SafeMessage", message, types);

    let mut parts = Vec::new();
    parts.push(hex::decode("1901").unwrap());
    parts.push(hex::decode(&domain_hash).unwrap());
    parts.push(hex::decode(&message_hash).unwrap());
    parts.concat()
}

fn hash_struct(
    primary_type: &str,
    data: &serde_json::Map<String, Value>,
    types: &serde_json::Map<String, Value>,
) -> String {
    let encoded = encode_data(data, primary_type, types);
    let result = my_keccak(&encoded)[2..].to_string();
    result
}

fn encode_data(
    data: &serde_json::Map<String, Value>,
    primary_type: &str,
    types: &serde_json::Map<String, Value>,
) -> Vec<u8> {
    let type_hash = hash_type(primary_type, types);
    let mut encoded_values: Vec<Value> = Vec::new();
    encoded_values.push(Value::String(hex::encode(&type_hash)));

    let type_fields = types.get(primary_type).unwrap().as_array().unwrap();
    for field in type_fields {
        let field_obj = field.as_object().unwrap();
        let field_type = field_obj.get("type").unwrap().as_str().unwrap();
        let field_name = field_obj.get("name").unwrap().as_str().unwrap();
        let value = data.get(field_name).unwrap();

        let encoded_field = encode_field(field_type, value);
        encoded_values.push(encoded_field);
    }

    let result = encode_abi_parameters(&encoded_values);
    result
}

fn encode_field(type_str: &str, value: &Value) -> Value {
    if type_str == "bytes" {
        let value_str = value.as_str().unwrap();
        if value_str.starts_with("0x") {
            let hex_str = &value_str[2..]; // Removes 0x
            let bytes = hex::decode(hex_str).unwrap();
            Value::String(my_keccak(&bytes))
        } else {
            value.clone()
        }
    } else {
        value.clone()
    }
}

fn hash_type(primary_type: &str, types: &serde_json::Map<String, Value>) -> Vec<u8> {
    let encoded_type = encode_type(primary_type, types);
    hex::decode(&my_keccak(encoded_type.as_bytes())[2..]).unwrap()
}

fn encode_type(primary_type: &str, types: &serde_json::Map<String, Value>) -> String {
    let fields = types.get(primary_type).unwrap().as_array().unwrap();
    let field_strs: Vec<String> = fields
        .iter()
        .map(|f| {
            let f_obj = f.as_object().unwrap();
            format!(
                "{} {}",
                f_obj.get("type").unwrap().as_str().unwrap(),
                f_obj.get("name").unwrap().as_str().unwrap()
            )
        })
        .collect();

    format!("{}({})", primary_type, field_strs.join(","))
}

fn encode_abi_parameters(values: &[Value]) -> Vec<u8> {
    let mut result = Vec::new();
    for v in values {
        result.extend(encode_abi_parameter(v));
    }
    result
}

fn encode_abi_parameter(v: &Value) -> Vec<u8> {
    let enc = match v {
        Value::Number(n) => {
            // Convert integers to 32-byte representation
            let n = n.as_u64().unwrap();
            let mut bytes = [0u8; 32];
            bytes[32 - 8..].copy_from_slice(&n.to_be_bytes());
            bytes.to_vec()
        }
        Value::String(s) => {
            if s.starts_with("0x") {
                // Convert hex strings to bytes
                let s = &s[2..]; // Remove '0x' prefix
                                 // Pad to 32 bytes (64 hex chars)
                let padded = format!("{:0>64}", s);
                hex::decode(padded).unwrap()
            } else {
                // Regular string - treat as hex string
                hex::decode(s).unwrap()
            }
        }
        Value::Array(arr) => {
            // Handle byte arrays
            let mut padded = vec![0u8; 32];
            for (i, b) in arr.iter().enumerate() {
                if i < 32 {
                    padded[i] = b.as_u64().unwrap() as u8;
                }
            }
            padded
        }
        _ => Vec::new(),
    };
    enc
}
