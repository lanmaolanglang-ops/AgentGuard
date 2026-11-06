# G0 攻击与回归验收矩阵

AgentGuard V2.0 · 2026-11-06 · 合成技术门禁

基准为固定 SQLite 3.54.1。拒绝例要求 SQL 语句或 COMMIT 真实失败，前后由独立读连接验证数据快照 SHA-256 及外键/领域完整性。正向例由独立连接确认写入。进程故障使用独立子进程读库与原七个强退夹具。缺任一证据、额外持久变化、发出错误 durable 回执即判失败。

| 编号 | 试例与预期结果 |
|---|---|
| G0a-01/02 | 改 Grant 主键、指向另一有效 Decision，语句拒绝 |
| G0a-03/04 | 删除 Grant 或 Decision，语句拒绝 |
| G0a-05/06 | 同 ID REPLACE Grant/Decision，语句拒绝 |
| G0a-07/08 | 有有效新 Decision 的 UPSERT 改绑、删除再插入，拒绝且无部分写入 |
| G0a-09 | rowid 冲突替换及不同 Grant 争用同一 Decision，拒绝 |
| G0a-10 | 合法创建另一 Grant，已确认提交并独立读出 |
| G0b-01/02/03 | 延长单一或双期限，拒绝 |
| G0b-04/05 | 缩短单期限或清空双期限，拒绝 |
| G0b-06/07/08 | Reserved 转另一合法状态时重设、Consumed 修改、同状态更新期限，拒绝 |
| G0b-09/10 | Pending 未登记 nonce 时单写期限拒绝；合法 Reserve 同时固定二者 |
| G0b-11/12 | 回拨后按单调限额失效，等于/超过单调截止均 expired、无 Execution |
| G0c-01/02/03 | 缺 nonce、他 Grant 的 nonce、重复 hash，拒绝 |
| G0c-04 | Reservation、nonce 和 Reserved 已写但缺审计，延迟外键 COMMIT 拒绝 |
| G0c-05/06 | 无 Execution 的 Consume 语句拒绝；已配 Execution 但缺消费审计，COMMIT 拒绝 |
| G0c-07/08 | 执行绑定错 Grant、配套审计与状态不一致，拒绝 |
| G0c-09 | 改 reservation/已绑定 execution 摘要、在错误摘要上建执行、Rust 用改动 Binding，全部拒绝 |
| G0c-10 | 原七个强退点，独立读出状态、nonce、双绑定与审计，未提交事务回滚 |
| G0c-11/12 | 延迟 FK 实际 COMMIT 故障不发 ACK，提交后响应故障 fenced 并仅允许对账 |
| G0c-13/14 | Worker 失败后退款或再次消费拒绝，旧归档改名重开拒绝激活 |

G0a-01 至 09 分别在递归触发器开启和关闭下重复。另覆盖 UTC 截止前、等于及之后，单调恰好与超过，旧 epoch 和墙钟前跳后回拨。领域检查的故障对照插入 SQL 可接受但业务不一致的孤立 Decision，writer 在 COMMIT 前回滚并 fenced。

原 I05 的 19 项测试保留。64 种状态对的正向 SQL 改为提供真实配套记录，并验证领域关系；15 合法、49 非法的期待不变。原损坏用例使用专门关闭外键的受控连接构造越界数据，再验证受支持连接重开拒绝，证明边界而非放宽保护。原 v1→v2 回归随向前迁移校验至 v3，保留原断言。

[测试结果](G0-TEST-REPORT.md) · [本轮脚本](../../tools/verify_g0.py)
