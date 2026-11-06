# G0 工程交付报告

AgentGuard V2.0 · 2026-11-06 · 独立分支 `p1-a/g0-ledger-hardening`

G0 加固以 I05 验收提交 `6f9bcf50ce5b327dc0225f612b5982990fe1716a` 为直接父基线。所有本地技术门禁通过，Windows CI 与提交绑定的回执见证据索引。负责人独立复核仍为最终验收门槛。

| 项目 | 工程结论 |
|---|---|
| G0a | PASS。Grant ID/Decision 稳定，删除、UPSERT 和 REPLACE 攻击拒绝，合法创建通过 |
| G0b | PASS。期限只在合法首次预占时写入，回拨、UTC 与单调边界验证通过 |
| G0c | PASS。Reservation/Nonce/Execution/Audit 闭环，缺配套的 COMMIT 失败，未知结果 fenced。64 状态对含 15 合法边 |
| 固定 SQLite | PASS。官方 3.54.1、Source ID、源码 SHA-256/SHA3-256、静态链接、禁扩展与连接配置实际核验 |
| 原 I05 / C2 / 第一批 | 原 I05 19 项、C2 探针、第一批 10 门禁及 44 项全部通过 |
| 新增测试 | 8 个新增函数、36 个指定编号；账本 27 通过、0 失败 |
| 冻结 | 42 份设计文件哈希一致，原 migration 001/002、原验收材料、C2 探针和 wire Schema 保留 |

变更限定于 [003 migration](../../crates/ledger/migrations/003-g0-integrity.sql)、账本连接/事务、原回归与 [G0 测试](../../crates/ledger/src/g0_tests.rs)、[验证入口](../../tools/verify_g0.py)、专项工作流及这套文档。既有 writer 模型和依赖锁定未改变。

G0 授权范围内没有待修的已知核心门禁失败。开放安全边界仍包括同主体任意 SQL/文件控制、恶意整体回滚、生产密钥、物理故障和 Windows 真实执行约束。对真实路线的进一步修复或设计由 I06、I07-B 和 D01–D05 独立审批。七款 Agent 保持 L0，五项能力均为 `false`。

建议提交负责人独立复核。I06 保持 BLOCKED，等待负责人下一条明确指令。[初始基线](G0-INITIAL-REVIEW.md) · [威胁分析](G0-THREAT-ANALYSIS.md) · [迁移方案](G0-MIGRATION-PLAN.md) · [验收矩阵](G0-ACCEPTANCE-MATRIX.md) · [安全审计](G0-SECURITY-AUDIT.md) · [测试报告](G0-TEST-REPORT.md) · [CI 与取证](G0-CI-EVIDENCE.md)
