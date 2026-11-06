# G0 Windows CI 与证据索引

AgentGuard V2.0 · 2026-11-06

专项工作流只在 `p1-a/g0-ledger-hardening` push 或显式 dispatch 时触发，Windows 2022、Rust/Cargo 1.97.1、Python 3.14.6、Node 24.19.0，固定 Actions SHA。ledger/G0 成功才进入 C2 和第一批，任何失败保留日志并退出。GitHub 归档仅包含测试合成日志与源代码对照。旧工作流保留第一批分支回归。[专项工作流](../../.github/workflows/p1-a-g0-ledger-hardening.yml) · [Windows 运行记录](https://github.com/lanmaolanglang-ops/AgentGuard/actions/workflows/p1-a-g0-ledger-hardening.yml?query=branch%3Ap1-a%2Fg0-ledger-hardening)

| 产物内容 | 复核方式 |
|---|---|
| `summary.json` | 绑定 head SHA，含原 I05 19 项、新增 8 个函数与 36 个编号、C2/第一批统计、SQLite 及 schema 摘要 |
| `ledger.log`、`ledger-evidence.json` | 完整账本/攻击日志、实际 Source ID/配置、独立 SQL 快照摘要、7 个子进程强退及迁移对比 |
| `c2.log`、`c2-evidence.json` | 原探针冷构建与 WAL/锁/强退/FULL 实测，两个阶段同一 SQLite amalgamation |
| `first-batch.log`、`first-batch/` | 原 10 项门禁与 17 Python、14 Rust、13 IPC 测试实际日志 |
| `schema-before.sql`、`schema-after.sql` | 001/002 原样快照与追加 003 后的 schema；运行时用 SQLite schema 模板独立校验 |
| `rust-writer.diff`、`frozen-design.json` | 相对 I05 的 Rust writer/db diff、42 份原始哈希清单与基线验证 |

固定源 `sqlite3.c` SHA-256 为 `0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52`，与 C2 的同一源码字节一致。官方公布的 SHA3-256 为 `e2b869d3003b33c942b7dd7063604fbc53553e1b66609570df39b9b7b558ee81`，两者分别记录。正式 I05/G0 沿用 OMIT_LOAD_EXTENSION，C2 原探针使用其已验收的独立编译选项。[SQLite 官方发布](https://sqlite.org/releaselog/3_54_1.html)

本地生成目录为 `.work/g0/`。最终远端回执将核对 run 的 head_sha、artifact.workflow_run 绑定、ZIP 实际 SHA-256 和 CRC，再读取独立证据计数。回执单独保存在本工作树 `.work/g0-ci-receipt.json`，避免给受测提交追加自引用。原 I05/C2 产物分别保留在各自验收运行。[原 I05 CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42202489196) · [原 C2 CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42192978038)

首次专项运行 `42228056699` 的账本产物实测 27 通过、0 失败。默认浅克隆缺少 I05 父提交，生成 Rust 差异证据时中止，未进入专项 C2 和第一批阶段。已调整专项工作流为完整检出，失败产物 `13505474488` 的 SHA-256 为 `915d8d542b80b15de71e32d9c2d3b22566c6d00f51c8964c4d6cf180cb6fa27f`，ZIP 和日志归档于 `.work/g0-ci-history/42228056699/`。[首次专项运行](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42228056699)
