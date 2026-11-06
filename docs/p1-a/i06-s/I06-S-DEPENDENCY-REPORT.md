# I06-S 依赖核验报告

**I06-S BLOCKED · S0 持久化接口门禁。** 2026-11-07，Asia/Shanghai。

## 负责人验收与代码基线

| 对象 | 核验结果 |
|---|---|
| I04 | 负责人已明确 ACCEPTED。远端验收提交为 `5f178223a702419cca56f63e62be775e2c62ca93`，对应 CI 成功。该提交在独立分支，未进入 I06-D 祖先链。 |
| I05 | 负责人已验收 `6f9bcf50ce5b327dc0225f612b5982990fe1716a`，对应 CI 成功，提交在 I06-D 祖先链。 |
| G0 | 负责人结论为 ACCEPTED WITH LIMITATIONS。远端分支和 Windows CI 均指向 `8c2a026468e05b352e4e3ed9de1db3dcc3ce8743`。 |
| I06-D | 本轮任务书给出负责人结论 ACCEPTED WITH LIMITATIONS。远端分支和 Windows CI 均指向 `4a1e900d71b3f197f2f2763816fa98800ce93feb`。本地验收工作树干净。 |
| 集成祖先 | G0 是 I06-D 的直接父提交。`git diff 8c2a026 HEAD -- crates/ledger` 为空，包含原 Writer、迁移 003 和一致性检查。未修改原分支。 |
| 冻结基线 | 实跑 `tools/baseline.py`，42 份设计文件逐字节一致。七款 Agent 为 L0，五项能力为 false。 |

[I04 CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42191949605) · [I05 CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42202489196) · [G0 CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42229482617) · [I06-D CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42250584224)

重新计算 I06-D 已归档 ZIP 的 SHA-256，得到 `7d3a0f741883a84283cbf239e9269f6c05129dcb8c360b50385aba244e2a3fbe`，与回执一致。归档显示 I06-D 24 项通过、21 组向量、6 组域载荷、G0 账本 27 项、C2 PASS 及第一批 44 项。该证据覆盖验收提交中的工程范围。历史 Preflight 的 BLOCKED 对应 G0 之前的账本约束缺陷。本轮检查以 G0 迁移 003 和现行事务为准。

本轮在独立工作树以验收 SHA 重跑 `tools/verify_i06_d.py`，得到同样的 I06-D 24 项、G0 账本 27 项、C2 PASS、第一批 44 项及冻结向量结果。输出保存在工作树的 `.work/s0-baseline`。这是既有基线的本地复验。I06-S 事务和进程验收项为 NOT VERIFIED。

## 设计偏差与阻断证据

| 缺口 | 现行实现 | 对 S1 的影响 |
|---|---|---|
| S0-01 事务输入及批准余额 | Writer 的 `Client::submit` 仅接收 `Mutation`。`Action::Reserve` 只有 nonce 哈希和 TTL，`Consume` 只有 nonce 哈希。Writer 在私有 `BEGIN IMMEDIATE` 事务中更新状态、写审计、验证领域约束并提交。I06-D 的批准、规则额度、reservation ID 和准备上下文均由内存模型持有。 | 现有入口无法在同一事务重核规则版本、批准、余额及准备绑定。把内存扣额与 G0 持久提交串联仍有两个状态变更点，进程终止时无法从账本重建二者的一致性。 |
| S0-02 摘要与截止点 | G0 的 `Binding.input` 在 Create 时写入不可变决策，003 同时将它保存在 `g0_reservations.operation_digest`。I06-D 的正式 operation MAC 要等到 Reserve 获得 nonce、reservation ID 和已签截止点才生成。G0 使用私有 `ProcessClock` 计算自己的期限，现有 `Receipt` 只有序号、Grant、状态、种类。 | 既有列继续表示 I05 合成输入摘要。将后生成的 HMAC 填入这个字段会违反原决策绑定。现有接口也无法证明落库期限与 Permit 的 UTC 和单调截止点相同。 |
| S0-03 消费和恢复 | D 的 `CommitObservation` 是内存故障模型。G0 的 `Outcome::Durable` 是提交确认，`Reconciled` 可读回原收据。G0 消费生成 `execution=unknown`，对账能读 Grant、基础 Execution 和 Receipt。 | 两边均无可供 S1 使用的原子启动响应登记，且无法按 S 所需的 Reservation／Peer／generation 对账。G0 历史收据只能作为对账结果。 |

[Writer](../../../crates/ledger/src/writer.rs) · [现有输入与收据](../../../crates/ledger/src/lib.rs) · [事务与时钟](../../../crates/ledger/src/store.rs) · [迁移 003](../../../crates/ledger/migrations/003-g0-integrity.sql) · [I06-D 状态机](../../../crates/typed-authorization/src/state.rs)

结论是来源与历史门禁通过，**现有已验收接口无法证明 S1 的原子兼容性**。这是阶段间集成缺口，G0 和 I06-D 现有验收结论各自有效。按任务书在 S0 停止，等待负责人确认是否批准 [新增事务接口提案](I06-S-IMPLEMENTATION-PLAN.md)。没有更改冻结合同、实现持久化桥接或启动 S1–S4。
