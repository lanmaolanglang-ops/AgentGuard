# G0 安全复核

AgentGuard V2.0 · 2026-11-06 · 技术复核，待负责人独立验收

## 原子性与写入路径

| 时点 | Rust/SQLite 执行与约束 |
|---|---|
| Create/Pend | 已验证的唯一 writer 在 `BEGIN IMMEDIATE` 内写入。Grant 初始 created 且无截止；状态更新需原状态已有相符审计，镜像 head 自动更新 |
| Reserve | 核对 pending、当前 epoch、eligible 和 input，计算原始双限。写不可变 Reservation → 写全库唯一 nonce → 写状态与双限 → 写同 Grant 审计。延迟外键在 COMMIT 检查 reserved 审计及 nonce |
| Consume | 检查 Reservation 与输入、decision、epoch、nonce、双限的完整对应。记录同 Grant 的执行绑定与 unknown 占位 → 更新 consumed → 写同 Grant 审计。缺占位在 SQL 状态迁移处拒绝，缺审计在提交期拒绝 |
| Cancel/WorkerFailed | execution 的关联和既有失败记录稳定，取消/失败必须匹配审计。执行失败保留 consumed 与 nonce，重复请求只对账或拒绝 |
| COMMIT | 原 writer 最终全域检查涵盖 FK、连续审计、状态头、Reservation/Nonce/Execution 绑定、截止及 cutoff。检查异常回滚。实际提交失败无 ACK，未知提交 fenced |

触发器只核对字段和当时可见前置条件。后写审计的跨语句配套由 SQLite `DEFERRABLE INITIALLY DEFERRED` 外键补齐，支持立即拒绝、事务内暂存配套和提交期拒绝三种可区分行为。当前状态头只能跟随真实 Grant，历史状态推进前须有前态审计，因此不能在同事务跳过某条前态审计。唯一索引 `audit_once` 支持按同 Grant/kind/state 建外键，序列检查禁止回填旧序号。

## 证明范围与风险

- 递归触发器 ON/OFF 下的改绑、删除、主键/rowid/唯一字段 REPLACE 均实测拒绝。受支持连接设置并读回 foreign_keys、recursive_triggers、trusted_schema，静态构建编译移除扩展加载。原始 SQL 连接若关闭外键等关键配置，G0 不能把其等同受控 writer。
- 3.54.1 上复现 v2 旧缺口后，v3 原始写入反例用新连接检查。失败前后比较包含 Decision、Grant、nonce、执行和审计的持久快照，外键与领域均通过。崩溃及未知例用独立子进程读库。
- 启动检查比对实际 schema 与锁定 migration，并校验领域。已确认不一致的历史库拒绝迁移，保留字节；形状完整却来源不明的旧未消费 Grant 失效，历史 consumed 保留。
- 字段名 `operation_digest` 目前保存 I05 合成输入摘要，不是签发 HMAC。实际权限绕过、同主体完整 SQL/文件控制、恶意整体回滚、物理掉电和真实 OS 隔离仍无证书。D01–D05 保持待决。

本轮 G0a/b/c 工程证据具备提交独立复核的范围。I06 及任何真实执行能力另需批准。七款 Agent 均 L0、五项能力标志均为 `false`。[威胁分析](G0-THREAT-ANALYSIS.md) · [证据](G0-CI-EVIDENCE.md)
