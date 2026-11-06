# I03节点验收

AgentGuard V2.0 · P1-A · 2026-11-06

## 当前结论

I03验收为PASS，限本批合成协议与IPC工程。I01/I02-A/I20-A前置已验收，I23也已完成来源调查与能力关闭。本地与GitHub Windows CI同入口10项门禁均通过，44个测试函数通过，失败0。

受测代码提交为9e8a7283b5b091f893d51d81af74b99458fbecfe，真实运行42152532076已完成且结论success。运行记录与artifact摘要见[CI回执](evidence/I03/github-ci.json)。[GitHub CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42152532076)

## 修改文件与架构

新增独立Rust crates/protocol与crates/core源文件、合同测试、tools/check_crypto.py、tools/ci.py、tools/test_ipc.py、.github/workflows/p1-a.yml及本节点文档。完整文件清单在本批变更索引提供。协议库负责strict parser、标准Schema、迁移和typed HMAC，Core独立进程仅承担有界合成校验。windows.rs集中只读身份及pipe FFI，合成probe为独立进程。

依赖沿用I20-A锁定版本与Cargo.lock，未新增UI/SQLite/Broker业务或Agent集成。正式Schema、状态机与字段所有权保持冻结。

## 本地测试与证据

统一入口为tools/ci.py，在项目.venv的Python中运行。10门禁分别为设计/身份/DAG、提取一致、Python测试、标准Schema、独立密码学参考、Rust格式、Clippy -D warnings、构建、Rust合同和真实合成IPC。

Python 17项、Rust 14项、Windows合成进程与CLI入口13项，共44个测试函数。最终执行结果以统一门禁和远端CI记录为准。65夹具、21向量、8迁移与64状态对是其中展开的数据覆盖，不重复计为额外测试函数。Rust/Python/Node对typed字节/frame/MAC一致。Core不会因收到审批、allow或OS身份字段而执行任何业务操作。

## 安全审查与未覆盖

固定中文错误不回显原始参数。OS实际peer PID+创建时间/token+image绑定和显式当前logon DACL用于合成进程通道。合成迁移只接受冻结8样本，真实历史日志导入等待I08脱敏。公网/其它用户/不同IL、真实Agent upstream身份、持久授权和Windows隔离边界仍未验证。签名、生产key部署、broker最终消费/崩溃恢复属于后续I05/I06/I07/I18/I22，当前不作认证。

对应Issue I03，AT21/23/29/34的上述合成子范围。完整Prepare→Evidence授权链需要I04/I05/I06，当前第一批按依赖停止。

[I03实现合同](I03-IPC-CONTRACT.md) [进度](PROGRESS.md)
