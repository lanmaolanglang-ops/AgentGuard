# G0 威胁分析

AgentGuard V2.0 · 2026-11-06 · 受控账本范围

资产包括历史 Grant→Decision 关联、准备后的双截止、原始授权绑定、全账本 nonce tombstone、执行占位、连续审计和唯一持久回执。威胁源包括有缺陷的内部迁移、受支持连接内的误用 SQL、并发/重放、进程中断与 SQLite 故障。攻击夹具使用静态链接 SQLite 3.54.1，全部数据位于独立临时库。

| 面 | 原路径及后果 | G0 约束 |
|---|---|---|
| G0a 决策重绑 | FK 只保证目标存在，UPDATE 能把 Grant 指向另一决策。OR REPLACE 在递归触发器关闭时可跳过删除触发器 | NULL-safe 保护主键、rowid、决策关联。禁止删除；BEFORE INSERT 检查主键、rowid 和唯一决策冲突。原始连接在递归 ON/OFF 都复验 |
| G0b 时间延长 | 双期限可在不改 state 的 UPDATE 里重写 | 比较真实值，仅在 Pending→Reserved、原双值为空且匹配同一 sealed Reservation 和 nonce 时允许设定。UTC/单调任一达到截止即失效 |
| G0c 不完整事务 | SQL 原状态触发器只校验单条状态边，原始连接可绕开 Rust 写入步骤 | 用受控事务和前置/后置触发器保持单行一致性；延迟外键负责提交期跨表闭环；全库领域检查在 writer COMMIT 前再次执行 |
| 恶意替换执行记录 | 缺少执行/审计配套、使用他 Grant 的 nonce 或不一致输入摘要 | Reservation 对齐 Grant、decision、epoch、nonce、截止；执行绑定对齐同 Grant 的 Reservation 与摘要，状态变更还需同 Grant 配套审计 |
| 故障与回退 | 在持久提交边缘崩溃，误把内存 ACK 当最终状态，或回滚后启用旧权限 | 强退点独立重开；未知 fenced，只能对账；Worker 失败仍 consumed；旧归档仅作历史，旧 epoch 不重新授权 |

正式绑定仅使用 I05 已存在的 32 字节合成输入摘要，SQL 字段 `operation_digest` 表示该内部承诺。实际 07/18 semantic/operation HMAC、身份、Permit、真实资源执行和能力认证另需批准。时间异常、物理满盘、硬件掉电与外部副作用不从数据库夹具推导。

受支持连接读回外键、递归触发器与 trusted_schema 设置。连接主动关闭外键时，SQLite 会失去延迟外键约束；原回归刻意用此不受支持设置污染临时库，重开在启动门闩拒绝。能够任意执行 SQL、DDL、停用触发器、控制 DB 文件或整体回退的同主体攻击者超出本次证明范围，D01/D05 保持待决。保护的产品价值是让用户所见批准、过期、撤销、已消费和未知结果可对账，避免把缺证据的状态展示成实际控制。

[迁移设计](G0-MIGRATION-PLAN.md) · [安全复核](G0-SECURITY-AUDIT.md)
