# G0 向前迁移与回滚

AgentGuard V2.0 · 2026-11-06 · 库版本 3

## 迁移顺序

1. 验证专用临时夹具范围、唯一 writer、SQLite Source ID、application_id、user_version 和非归档状态。原版本只接受 1、2、3。写入前按实际旧版本的 001/002 模板验证结构、`integrity_check`、外键和领域约束。
2. 在 `BEGIN IMMEDIATE` 内重验，必要时先执行 002，再重验 Grant→Decision、审计链、历史 Reserved 与 nonce/双期限的存在性。数据库中没有足够证据逆推任一旧截止的原始可信值。
3. 保留旧 Consumed 和 Execution。重启生成新 boot epoch，将旧 created/pending/reserved 原子失效并写真实 invalidated 审计。已保留的 nonce 不删除、不重用。历史数据不一致时返回 Corrupt，不写迁移，保留原文件供隔离只读审查。
4. 以 [003 migration](../../crates/ledger/migrations/003-g0-integrity.sql) 建新表、视图和触发器，保存审计切换点及当前状态索引。历史记录无需也不会补造 Reservation、Nonce、Execution 或 Audit。schema 版本升至 3。
5. 同一事务完成前后模板和领域复核，确认 COMMIT 后才开放 writer。独立连接重开，核对各表与历史记录。v3 后续正常重启也用相同校验与失效策略。

| 独立取证 | 旧 v2 | 新 v3 |
|---|---|---|
| 活动旧预占 | reserved，3 条审计 | invalidated，4 条审计，旧 boot 请求拒绝 |
| 已消费历史 | consumed、4 条审计、execution unknown | 状态、4 条审计和 execution 原样 |
| nonce | 2 条 | 仍 2 条，历史 G0 reservation/execution 绑定新增 0 |
| 不一致副本 | v2 人工写入与审计不一致的状态 | 重开 Corrupt，版本保持 2，原数据库字节及 3 条审计保留 |

另有原 v1 夹具迁移至 3、归档只读及改名尝试激活失败的回归。对模板比较只折叠 CRLF/LF，支持同一 SQL 在不同 Git checkout 的换行差异。

## 回滚与现场保留

迁移失败回滚整个事务，保留旧库、WAL 与失败日志。未知提交先通过独立连接核对 schema 版本、epoch 和审计，不尝试原请求再授权。代码可回到已验收版本进行只读诊断，活动数据按版本隔离；旧代码对 v3 库因版本未知而拒绝。归档快照可供独立历史审查，禁止恢复成活动状态或恢复过期/已消费的授权。异常 fixture 文件仅在发生测试 panic 时保留，正常用例清理其专用临时目录。

[测试报告](G0-TEST-REPORT.md) · [独立证据说明](G0-CI-EVIDENCE.md)
