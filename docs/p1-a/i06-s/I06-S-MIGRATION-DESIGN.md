# I06-S 追加迁移、恢复与回退

**提案，当前没有迁移文件或数据库变更。** 计划为 S 显式模式增加版本 4。只在新建、带 fixture 标记的本地固定盘临时目录中，使用独立测试副本验证。源库、原归档及验收材料以哈希校验并保留。[G0 迁移 003](../../../crates/ledger/migrations/003-g0-integrity.sql) · [G0 版本与一致性实现](../../../crates/ledger/src/db.rs)

## 追加记录与约束

| 拟增对象 | 权威记录和约束 |
|---|---|
| `s_meta`、`s_activation_history` | fixture lineage、启动 fence 世代、Core boot epoch、Writer clock domain、受信 OS boot 标识、key ID／generation。一个活动头，历史仅追加。policy/grant epoch 复用 G0 meta。若 I06-D epoch 超出 G0 `u32`，显式拒绝。归档和 fence 不匹配禁止写路径。 |
| `s_grants`、`s_prepared` | Grant 主键引用现有 `grants`，绑定同一 Decision、invocation、Prepared、原连接与 Broker／Worker 实例、版本、epoch、语义及 manifest 摘要、双时限和时钟域。原 Grant 状态仍只有 `grants.state`。准备事实不可变，失效由 Writer 追加证据并封锁旧版本。 |
| `s_rule_versions`、`s_rule_heads` | 规则版本和精确范围不可变，头记录当前版本、撤销与计数。按规则 ID 维护一份跨版本用量。变更版本须有新的显式规则批准且额度上限不得小于已用量。禁止自动把旧用量清零或重新启用已撤销规则。 |
| `s_approval_versions`、`s_approval_heads`、`s_approval_claims` | 绑定已校验的合成批准者、挑战／确认事件、批准版本、决定、Grant、invocation、Prepared、快照、规则引用及双期限。单次认领以 approval ID 为主键，另对 Reservation ID 唯一。活动版本和撤销由 Writer 更新，历史与 claim 不可删除。 |
| `s_reservations` | Reservation ID 主键，Grant ID 唯一。通过 FK 对应既有 G0 Reservation、nonce、认领及审计。保存 immutable binding、`operation_mac`、`permit_mac`、key ID／generation、时钟域、签发样本和双截止点。使用同一 G0 nonce 主键，S 行的 nonce hash 唯一且必须对应同一 Grant。原 nonce、Permit、密钥、原始秘密和操作参数不入库。 |
| `s_launch_once`、`s_execution_facts` | 启动登记对 Reservation、Grant、launch ID 均唯一，FK 绑定 G0 execution、S Reservation 和审计，保存原连接、代次、期限与响应指纹。启动证据、取消请求／确认、Worker 自报和独立证据是分开追加的事实。执行投影源自这些事实与现有 `execution` 行，Grant 终态独立。无发件箱或可查询的响应正文。 |
| `s_audit`、`s_requests` | 脱敏审计带有闭合 kind 和序号。涉及 G0 转移的 S 审计引用同一事务的 G0 audit seq，规则登记等 S 独有控制事件有自身审计。S 请求收据按 request ID 唯一并引用审计与相关业务行，保存规范指纹和 commit 事实，禁止重建出站响应。 |

数据库约束采用 STRICT、非空、范围及固定字节长度检查、唯一键、组合外键、不可变记录和受控追加。需要双向关联的 FK 延迟至 COMMIT，其他 FK 默认立即验证。跨表语义由 Writer 在同一事务中核验。S 模式继续启用外键、recursive triggers、trusted_schema=OFF、WAL/FULL，签名在 Rust 执行。历史 MAC 缺少原密钥时只能核验审计结构和链接元数据。[SQLite WAL 提交语义](https://www.sqlite.org/wal.html)

不变量用于事务核验和独立只读取证：

1. S Grant 使用同一 G0 决策绑定。成功 Reserve 在同一个 Reserve 审计序号下关联恰好一份 claim、一行 S Reservation、一个全局唯一 nonce 和一条收据。两个截止点与 G0 Reservation 完全相同。以有规则的已提交 claim 计数与该规则用量逐一核对，撤销、过期和消费失败均不减少历史用量。
2. S Grant 的 consumed 必须同时存在 G0 execution、恰好一条启动登记及对应审计和请求收据。审计或外键缺失使整个事务失败。启动登记的存在只证明提交了可签发条件，送达或执行需要另有证据。
3. S 请求写入 `s_requests`，关联的旧 G0 audit 行保持原语义且 request 字段为空。两入口在 v4 内互查请求 ID 命名空间。同 ID 的冲突不进入另一入口。旧 G0 写入口对 S 归属 Grant（含历史终态及新写入冲突）拒绝，S 入口只处理显式 S 归属。控制、重启、归档等内部路径必须保持同一审计不变量。
4. `launch_id` 及历史 nonce／claim 无重置路径。G0 输入摘要分列保留。S 存储的 HMAC 只对应 07/18，任何查询返回的行均无发布权。

## 安装、验证与代码回退

1. G0 的 003 安装路径显式锁在版本 3，原 SQL 文件不变。v4 只在 S fixture 模式通过应用 ID、schema 指纹、G0 领域约束及测试标记核验后，经同一 Writer 初始化追加。结构参考库按版本依次加载 001–003 与拟议追加迁移，拒绝缺表、额外对象或未知版本。先覆盖旧版安装分支，避免全局 `VERSION=4` 将 003 安装误标为 4。
2. 迁移与版本标记、旧未完成 Grant 的失效及审计在一个受控初始化事务内提交。启用前验证 `integrity_check`、`foreign_key_check`、G0/S 领域一致性、schema 指纹、测试 Marker 和活动头。版本 3 记录无 S 归属，也无从旧收据晋升成 Permit 的路径。新模式关停时 v4 只允许审计读取或明确拒绝打开。
3. DDL、域检查、故障注入或 COMMIT 失败后不启用。确认回滚后原测试副本仍为 3，确认未知时先封存并独立读回版本和完整性，不能只读 `user_version` 就重新启用。
4. 代码回退保留 v4 库、审计、旧备份、Marker 和失败日志。旧 G0 可执行文件只用未迁移的独立 v3 fixture，并按 G0 规则重新开账及失效旧未完成 Grant。v4 用兼容只读工具核验，禁止强降 `user_version`、DROP S 表或重写历史。回退测试比较迁移前源库 SHA-256、旧路径回归、v4 只读可见性和旧程序拒绝写 v4。

## 双截止点、启动 Fence 与受控恢复

- 运行期间时钟域为当前 Writer activation 的单调基准，OS boot、Core epoch 和 Writer clock domain 分别校验。正常重启、同 Core 内 Writer 重启及密钥轮换均先关闭旧队列和 reply slot。原时钟域终止，绝不比较跨域 tick。新激活只从新的 UTC／mono 样本开始。旧的开放 Grant 由同一启动事务走 G0 失效及 S 配套审计，已消费保留，未决执行只登记 UNKNOWN 的恢复观察。旧批准、规则和 Prepared 均无当前 epoch 权力。
- 独立测试 Fence Marker 保存在数据库和备份之外。受控 harness 指定经过校验的 fixture lineage 路径与测试来源。Writer 的启动所有者独占更新 Marker 和 DB 激活头，完成初始化后把同一个数据库连接移交 Writer 队列，整个过程只有一个写入所有者。Marker 至少含 lineage、单调递增 activation 序号、计划激活标识和已知归档来源。独占 lineage 锁覆盖 Marker 核验到关闭 Writer，禁止两个副本在同一 lineage 并行激活。
- 启动先将 Marker 提前至新的激活标识，再由唯一 Writer 在 SQLite 事务写匹配的活动头、epoch、恢复失效和审计。COMMIT 成功且重新核对 Marker 与 DB 标识一致后才开放。Marker 写失败或任一侧未知则关闭。两者之间崩溃留下不匹配，须按受控流程复核并进行下一次新激活，禁止根据旧 DB 下调 Marker。外部 Marker 只约束激活，Reserve/Consume 不为它增加第二个提交点。
- 备份使用现有 G0 备份语义，指定只读归档与来源校验。旧备份恢复试验在另一个 fixture 副本检查 Marker、版本、G0/S 审计与完整性。旧归档保持历史只读，原 Reservation 或 Permit 再提交按旧 generation／fence 拒绝。原活动库的完整 nonce、claim 和收据历史继续保留。只有旧备份时不能证明后续消费、计数或执行结果，该 lineage 进入历史对账模式。负责人若授权另起独立 fixture lineage，须使用新键、新 epoch 和新批准，旧库和 Marker 留存。此策略放弃从不完整历史继续授权。[SQLite 备份机制](https://www.sqlite.org/backup.html)

上述流程的目标是验证受控恢复。测试 Marker、SQLite 事务与本地锁不能证明同一安全主体同时回滚 Marker 和数据库时仍然安全。D05 的生产锚、真实可信时钟和物理故障恢复需独立验证。[时序与对账](I06-S-TRANSACTION-SEQUENCE.md) · [故障矩阵](I06-S-ATOMICITY-FAULT-MATRIX.md)
