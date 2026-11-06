//! Separate synthetic client. It cannot connect to or configure any real Agent.
use agentguard_protocol::{ProtocolError, Result};
use serde_json::json;

#[cfg(windows)]
async fn run() -> Result<()> {
    use agentguard_core::{ipc, windows};
    use tokio::net::windows::named_pipe::ClientOptions;
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err(ProtocolError::new("usage", "合成客户端参数不完整。"));
    }
    windows::validate_pipe_name(&args[1])?;
    let expected_pid: u32 = args[2]
        .parse()
        .map_err(|_| ProtocolError::new("pid", "服务端进程编号无效。"))?;
    let started = tokio::time::Instant::now();
    let mut pipe = loop {
        match ClientOptions::new().open(&args[1]) {
            Ok(pipe) => break pipe,
            Err(_) if started.elapsed() < ipc::DEADLINE => {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await
            }
            Err(_) => {
                return Err(ProtocolError::new(
                    "connect_failed",
                    "无法连接指定合成Core。",
                ));
            }
        }
    };
    let actual_pid = windows::server_pid(&pipe)?;
    let peer = windows::Identity::capture(actual_pid)?;
    let current = windows::Identity::capture(std::process::id())?;
    if actual_pid != expected_pid || peer.image_sha256 != args[3] || !current.same_principal(&peer)
    {
        return Err(ProtocolError::new(
            "server_identity",
            "服务端身份不匹配，已停止发送请求。",
        ));
    }
    let hello = ipc::read_json(&mut pipe).await?;
    if hello["kind"] != "synthetic_hello"
        || hello["major"] != 2
        || hello["minor"] != 0
        || hello["challenge"]
            .as_str()
            .is_none_or(|value| value.len() != 64)
    {
        return Err(ProtocolError::new("hello", "服务端握手不符合协议。"));
    }
    peer.revalidate()?;
    let mut request = json!({"major":2,"minor":0,"sequence":1,"challenge":hello["challenge"],
        "method":"report_event","payload":{"source_event_id":"synthetic-1","agent_id":"A01",
        "tool_name":"fixture-read","command":null,"arguments":{"path":"virtual.txt"}}});
    match args[4].as_str() {
        "valid" | "duplicate" | "conflict" => {}
        "forged_allow" => request["payload"]["policy_decision"] = json!("allow"),
        "forged_identity" => {
            request["payload"]["principal_snapshot"] = json!({"status":"os_verified"})
        }
        "evaluate" => {
            request["method"] = json!("evaluate");
            request["payload"] = json!({});
        }
        "replay" => request["sequence"] = json!(0),
        "wrong_challenge" => request["challenge"] = json!("old-connection"),
        "stall" => {
            tokio::time::sleep(ipc::DEADLINE + std::time::Duration::from_secs(1)).await;
            return Ok(());
        }
        "truncated" => {
            use tokio::io::AsyncWriteExt;
            pipe.write_all(&128u32.to_be_bytes())
                .await
                .map_err(|_| ProtocolError::new("write", "测试发送失败。"))?;
            pipe.write_all(b"{")
                .await
                .map_err(|_| ProtocolError::new("write", "测试发送失败。"))?;
            return Ok(());
        }
        "oversized" => {
            use tokio::io::AsyncWriteExt;
            pipe.write_all(&1_048_577u32.to_be_bytes())
                .await
                .map_err(|_| ProtocolError::new("write", "测试发送失败。"))?;
            return Ok(());
        }
        _ => return Err(ProtocolError::new("scenario", "未知合成测试场景。")),
    }
    ipc::write_json(&mut pipe, &request).await?;
    let mut response = ipc::read_json(&mut pipe).await?;
    if matches!(args[4].as_str(), "duplicate" | "conflict") {
        request["sequence"] = json!(2);
        if args[4] == "conflict" {
            request["payload"]["arguments"] = json!({"path":"changed"});
        }
        ipc::write_json(&mut pipe, &request).await?;
        response = ipc::read_json(&mut pipe).await?;
    }
    println!("{}", response);
    Ok(())
}

#[cfg(not(windows))]
async fn run() -> Result<()> {
    Err(ProtocolError::new(
        "windows_required",
        "这个合成IPC检查需要Windows。",
    ))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{}", serde_json::to_string(&error).unwrap());
        std::process::exit(1);
    }
}
