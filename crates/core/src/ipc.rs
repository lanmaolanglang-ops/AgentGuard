use agentguard_protocol::{
    ProtocolError, Result,
    parse::{MAX_FRAME_BYTES, parse_json},
};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{Duration, timeout};

pub const DEADLINE: Duration = Duration::from_secs(5);

pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<u8>> {
    timeout(DEADLINE, async {
        let mut length = [0; 4];
        reader.read_exact(&mut length).await.map_err(|_| {
            ProtocolError::new("ipc_closed", "连接已中断，本次请求没有获得执行许可。")
        })?;
        let length = u32::from_be_bytes(length) as usize;
        if length == 0 || length > MAX_FRAME_BYTES {
            return Err(ProtocolError::new(
                "frame_limit",
                "请求超过协议限额，本次请求已拒绝。",
            ));
        }
        let mut bytes = vec![0; length];
        reader
            .read_exact(&mut bytes)
            .await
            .map_err(|_| ProtocolError::new("ipc_truncated", "请求未完整送达，本次请求已拒绝。"))?;
        Ok(bytes)
    })
    .await
    .map_err(|_| ProtocolError::new("ipc_timeout", "连接等待超时，本次请求没有获得执行许可。"))?
}

pub async fn write_json<W: AsyncWrite + Unpin>(writer: &mut W, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| ProtocolError::new("response", "无法生成校验结果。"))?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::new("frame_limit", "校验结果过大。"));
    }
    timeout(DEADLINE, async {
        writer
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .await?;
        writer.write_all(&bytes).await?;
        writer.flush().await
    })
    .await
    .map_err(|_| ProtocolError::new("ipc_timeout", "连接等待超时。"))?
    .map_err(|_| ProtocolError::new("ipc_closed", "连接已中断。"))
}

pub async fn read_json<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Value> {
    parse_json(&read_frame(reader).await?)
}
