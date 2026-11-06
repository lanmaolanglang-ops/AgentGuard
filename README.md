# AgentGuard

Windows优先的多Agent行为解释、监控和风险权限管理。当前工程阶段为P1-A第一批合成验证。七款Agent均为L0，五项能力标志全部false。

## 设计基线

[规范索引](docs/p0.1/P0.1-DOCUMENT-INDEX.md)与[本批范围](docs/p1-a/SCOPE-AND-ACCEPTANCE.md)。docs/p0与docs/p0.1固定只读。

## 构建、测试与运行

Windows PowerShell，在项目根目录运行。

~~~powershell
uv venv --python 3.14.6 .venv
uv pip install --python .venv/Scripts/python.exe --require-hashes --only-binary=:all: --index-url https://pypi.org/simple -r requirements-ci.lock
.venv\Scripts\python.exe -B tools/ci.py
cargo build --workspace --locked
cargo test --workspace --locked
.venv\Scripts\python.exe -B tools/test_ipc.py
~~~

需要Rust 1.97.1、MSVC BuildTools、Python 3.14.6和Node 24.19.0。CI固定同一工具版本并运行同一入口。

独立Core的validate命令可检查合成JSON，调用形式为target\debug\agentguard-core.exe validate event-v2 <JSON路径>。合成命名管道由tools/test_ipc.py启动Core与独立probe验证，结果始终authorization_available=false。

Tauri 2/React/TypeScript界面属于I09，SQLite ledger属于I05，完整授权与Broker属于I06。本批交付协议与合成校验入口，七款真实接入和OS隔离仍待后续认证。

[进度与证据](docs/p1-a/PROGRESS.md)
