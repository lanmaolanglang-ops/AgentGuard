use agentguard_protocol::{
    ProtocolError, Result,
    ingress::{Role, Session},
    parse::parse_json,
    schema::Schemas,
};
use serde_json::json;
use std::sync::Arc;

#[cfg(windows)]
async fn serve(name: &str, expected_client_hash: &str) -> Result<()> {
    use agentguard_core::{ipc, windows};
    let schemas = Arc::new(Schemas::new()?);
    let current = windows::Identity::capture(std::process::id())?;
    let pipe = windows::create_pipe(name, &current)?;
    tokio::time::timeout(ipc::DEADLINE, pipe.connect())
        .await
        .map_err(|_| ProtocolError::new("connect_timeout", "等待合成客户端超时。"))?
        .map_err(|_| ProtocolError::new("connect_failed", "合成连接失败。"))?;
    let pid = windows::client_pid(&pipe)?;
    let peer = windows::Identity::capture(pid)?;
    if !current.same_principal(&peer) || peer.image_sha256 != expected_client_hash {
        return Err(ProtocolError::new(
            "peer_mismatch",
            "连接进程与允许的合成客户端不一致，已拒绝连接。",
        ));
    }
    let challenge = windows::challenge()?;
    let mut session = Session::new(Role::Adapter, challenge.clone(), schemas);
    // Splitting the pipe permits bounded read/write without creating an execution channel.
    let (mut reader, mut writer) = tokio::io::split(pipe);
    ipc::write_json(
        &mut writer,
        &json!({"kind":"synthetic_hello","major":2,"minor":0,"challenge":challenge}),
    )
    .await?;
    loop {
        let bytes = match ipc::read_frame(&mut reader).await {
            Ok(bytes) => bytes,
            Err(error) if error.code == "ipc_closed" => return Ok(()),
            Err(error) => return Err(error),
        };
        peer.revalidate()?;
        let result = session.accept(&bytes);
        let response = match &result {
            Ok(value) => json!({"ok":true,"result":value,"error":null}),
            Err(error) => json!({"ok":false,"result":null,"error":error}),
        };
        ipc::write_json(&mut writer, &response).await?;
        if result.is_err() {
            return Ok(());
        }
    }
}

async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("validate") if args.len() == 4 => {
            use std::io::Read;
            let file = std::fs::File::open(&args[3])
                .map_err(|_| ProtocolError::new("input_file", "无法读取指定合成输入。"))?;
            let mut bytes = Vec::new();
            file.take(agentguard_protocol::parse::MAX_FRAME_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| ProtocolError::new("input_file", "无法读取指定合成输入。"))?;
            Schemas::new()?.validate(&args[2], &parse_json(&bytes)?)?;
            println!(
                "{}",
                json!({"status":"schema_valid","executed":false,"authorization_available":false})
            );
            Ok(())
        }
        #[cfg(windows)]
        Some("serve-synthetic") if args.len() == 4 => serve(&args[2], &args[3]).await,
        _ => Err(ProtocolError::new(
            "usage",
            "用法为validate <schema> <合成JSON路径>，或serve-synthetic <专用管道> <合成客户端SHA256>。",
        )),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{}", serde_json::to_string(&error).unwrap());
        std::process::exit(1);
    }
}
