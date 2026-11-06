use crate::parse::{MAX_DEPTH, MAX_FRAME_BYTES, MAX_INTEGER};
use crate::{ProtocolError, Result};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

#[derive(Clone, Copy, Debug)]
pub enum Domain {
    Semantic,
    Operation,
    Intent,
    Secret,
    Permit,
    Audit,
}

impl Domain {
    pub fn name(self) -> &'static str {
        match self {
            Self::Semantic => "semantic-v2",
            Self::Operation => "operation-v2",
            Self::Intent => "intent-v2",
            Self::Secret => "secret-v2",
            Self::Permit => "permit-v2",
            Self::Audit => "audit-v2",
        }
    }
}

fn budget_error() -> ProtocolError {
    ProtocolError::new("encoding_limit", "操作内容超过编码限额，本次请求已拒绝。")
}

fn append(output: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
    if output.len().saturating_add(bytes.len()) > MAX_FRAME_BYTES {
        return Err(budget_error());
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn count(output: &mut Vec<u8>, length: usize) -> Result<()> {
    let length = u32::try_from(length).map_err(|_| budget_error())?;
    append(output, &length.to_be_bytes())
}

fn encode(value: &Value, depth: usize, output: &mut Vec<u8>) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(budget_error());
    }
    match value {
        Value::Null => append(output, &[0]),
        Value::Bool(false) => append(output, &[1]),
        Value::Bool(true) => append(output, &[2]),
        Value::Number(number) => {
            let integer = number
                .as_i64()
                .filter(|n| (-MAX_INTEGER..=MAX_INTEGER).contains(n))
                .ok_or_else(|| {
                    ProtocolError::new("unsafe_integer", "操作包含不支持的数字，本次请求已拒绝。")
                })?;
            append(output, &[3])?;
            append(output, &integer.to_be_bytes())
        }
        Value::String(text) => {
            append(output, &[4])?;
            count(output, text.len())?;
            append(output, text.as_bytes())
        }
        Value::Array(values) => {
            if values.len() > 128 {
                return Err(budget_error());
            }
            append(output, &[6])?;
            count(output, values.len())?;
            for value in values {
                encode(value, depth + 1, output)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            if values.len() > 64 {
                return Err(budget_error());
            }
            append(output, &[7])?;
            count(output, values.len())?;
            let mut keys: Vec<_> = values.keys().collect();
            keys.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
            for key in keys {
                append(output, &[4])?;
                count(output, key.len())?;
                append(output, key.as_bytes())?;
                encode(&values[key], depth + 1, output)?;
            }
            Ok(())
        }
    }
}

pub fn typed_encode(value: &Value) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    encode(value, 0, &mut output)?;
    Ok(output)
}

pub fn frame(domain: Domain, value: &Value) -> Result<Vec<u8>> {
    let mut output = b"AG2".to_vec();
    append(&mut output, &(domain.name().len() as u16).to_be_bytes())?;
    append(&mut output, domain.name().as_bytes())?;
    append(&mut output, &typed_encode(value)?)?;
    Ok(output)
}

/// Caller owns a single-purpose key. This library is not exposed as an IPC signing service.
pub fn mac(key: &[u8; 32], domain: Domain, value: &Value) -> Result<[u8; 32]> {
    let mut signer = Hmac::<Sha256>::new_from_slice(key)
        .map_err(|_| ProtocolError::new("key_error", "密钥不可用，本次请求已拒绝。"))?;
    signer.update(&frame(domain, value)?);
    Ok(signer.finalize().into_bytes().into())
}

pub fn decode_mac(text: &str) -> Result<[u8; 32]> {
    if text.len() != 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ProtocolError::new(
            "mac_format",
            "授权校验码格式无效，本次请求已拒绝。",
        ));
    }
    let mut bytes = [0; 32];
    for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
        let digit = |byte: u8| {
            if byte.is_ascii_digit() {
                byte - b'0'
            } else {
                byte - b'a' + 10
            }
        };
        bytes[index] = digit(pair[0]) * 16 + digit(pair[1]);
    }
    Ok(bytes)
}

pub fn verify(key: &[u8; 32], domain: Domain, value: &Value, supplied: &str) -> Result<()> {
    let expected = decode_mac(supplied)?;
    let mut verifier = Hmac::<Sha256>::new_from_slice(key)
        .map_err(|_| ProtocolError::new("key_error", "密钥不可用，本次请求已拒绝。"))?;
    verifier.update(&frame(domain, value)?);
    verifier.verify_slice(&expected).map_err(|_| {
        ProtocolError::new("mac_mismatch", "操作内容与授权记录不一致，本次请求已拒绝。")
    })
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
