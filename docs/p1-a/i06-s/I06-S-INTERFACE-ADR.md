# I06-S 接口 ADR · 单 Writer 最终验签与提交

**S0 INTERFACE DESIGN READY FOR OWNER REVIEW** · 2026-11-07 · 提案，待负责人审查。

## 基线与决策

G0 `8c2a026` 的 `Ledger::apply_inner` 已使用独占 Writer 和 `BEGIN IMMEDIATE` 提交状态、审计及收据。I06-D `4a1e900` 已验证冻结 Schema、六域 MAC 和内存状态机。I06-D 的 `CommitObservation` 只表示合成内存结果。G0 的 `Outcome::Reconciled` 只返回历史收据。两者之间目前缺少可证明原子性的接口。[依赖核验](I06-S-DEPENDENCY-REPORT.md) · [G0 写事务](../../../crates/ledger/src/store.rs) · [I06-D 状态机](../../../crates/typed-authorization/src/state.rs)

| 架构 | 可行性及代价 | 决策 |
|---|---|---|
| A · 同进程 Writer 内最终验签 | Core 持有密钥。现有 Writer 线程接收封闭的 Core 授权上下文，在读到事务内状态和时钟样本后，调用 I06-D 的受限密码学接口。无事务内 IPC。需在同一可信进程内限制签名入口和密钥访问。 | 采用。避免凭事务外快照确定签名期限或验签结果，现有单 Writer 是唯一提交点。 |
| B · Core 预签及事务内见证复核 | Writer 先发当前 epoch／key generation／绑定及双时限挑战。Core 密钥模块按冻结合同签名或验签，并返回仅该模块可构造、不可克隆的见证。见证绑定挑战、完整候选摘要及已验票据摘要。Writer 在最终事务重读并核对每个字段，确认挑战未使用且未过期，任何变化使候选报废。密钥访问范围更小，但两次排队和额外挑战生命周期消耗期限。 | 同进程可行。若分进程还需另审见证的认证传输。采用 A 避免增加挑战协议与跨时间样本复核。 |

## 拟增接口

以下名称和类型为内部设计。所有可承载权利的类型私有构造、不可反序列化、不可克隆、不可调试打印。参数中的角色、时钟和 Peer 由 Core 信任边界注入。

| 入口或类型 | 方向和内容 | 输出或权力边界 |
|---|---|---|
| `preflight_s_reserve` | Core → I06-D。严格解析冻结快照，校验 recipe、硬禁、原生许可、身份、已冻结 Prepared。输入 `RequestId`、Grant／Decision／invocation、人工批准 ID＋版本、显式 `OneShot` 或 `Parent(rule_id,version)`、TTL、Core 捕获的 Peer 与 `PreparedLease`。 | `PrecommitCandidate<Reserve>`。绑定原始已验证数据或其不可变短期引用。可读缓存用于预筛选，余额、授权状态、nonce 和审计均未变更。 |
| `preflight_s_consume` | Core → I06-D。输入冻结 `permit-v2` 原始有界字节、当前 Prepared 快照及同一受信会话证据。严格解析，检查结构并生成待验候选。 | `PrecommitCandidate<Consume>`。可做快速密码学拒绝。最终 MAC 与持久状态在 Writer 内复验。 |
| `Client::submit_s` | 唯一 Writer 的受控队列。封闭 `SCommand` 分为 Reserve、Consume、登记 Grant／Decision／Prepared、登记和撤销规则与批准、准备失效、取消及恢复控制。请求含当前 activation、三种 epoch、key generation、连接序号与不可变命令指纹。 | Reserve 和 Consume 各自返回 `FreshCommit`、`StatusOnly`、`Rejected` 或 `Unknown(request_id)`。其他变更只返回状态或错误。入口不接受原始 SQL、通用回调或调用方声明的验签布尔值。 |
| `VerifiedCandidate<T>` | 仅 Writer 在本次事务读、最终验证后构造。类型绑定事务生命周期、使用过的 clock sample、版本和 peer lease。I06-D 提供纯验证与原合同 payload 组装，现有公开方法仍运行原测试模型。 | 可生成事务内临时的待发送字节。事务离开时即丢弃该类型。S 不调用原内存 `reserve`／`consume` 去推进另一份 Grant 或额度。 |
| `DurableCommitReceipt<T>` | Writer 的 COMMIT 返回成功、事务收据已绑定对应审计后产生。包含 operation kind、request ID、Grant、Reservation、审计序号、activation、最终状态、可选 launch ID。 | 新提交回执。收到这一回执是发送的必要条件。历史查询只能产生不同类型的 `ReconcileView`。 |
| `FreshCommit<T>` / `EgressOnce<T>` | 唯一原请求的本地 reply slot 持有匹配的耐久回执及可选一次性响应。Core 复核连接、实例、generation、epoch、lease 和期限后，按所有权消费响应。 | Reserve 可发送冻结 Permit，Consume 可返回一次 `LaunchAuthorization`。响应可因关门、过期或连接中断而丢弃。状态仍已提交，后续查询仅能获得状态。 |
| `Inspection::reconcile_s` | 仅 Core 使用的只读单快照查询。按请求和绑定读取已脱敏状态，检查角色及作用域。 | `ReconcileView`，包含可见 Grant、Reservation 摘要、已提交收据、执行事实及证据等级。禁止含 raw nonce、Permit、启动响应、密钥或可重建它们的原始快照。 |

既有 G0 `Client::submit`、`Ledger::apply`、迁移 001–003、I06-D 公开 API 和 07/15/18 合同继续作为兼容边界。后续 S1 只在现有 Writer 加封闭命令、迁移版本识别、S 事务及只读查询，在 I06-D 加受限纯验证入口。S 专用 fixture 显式启用。原入口对旧库与旧 Grant 保持行为，S Grant 的旧入口写操作必须被路由护栏拒绝。[迁移与作用域](I06-S-MIGRATION-DESIGN.md)

## 权威状态与证据

- Writer 持有唯一 SQLite 写连接及原 fixture 锁。Grant 只使用 G0 的八状态和既有转移约束，S 记录引用同一行与同一审计。数据库中的 Parent Rule 有效版本、撤销事实、单次意图和额度计数是权威，缓存只有预筛选和拒绝用途。Execution 与 Grant 分开，来自既有执行行和 S 事件证据。
- 人工批准由受信合成 UI 的显式操作触发，Core 校验角色与会话、一次性挑战、实际展示的冻结快照及选择结果，由 Writer 登记批准版本和审计。批准绑定批准者上下文、Decision、Grant、invocation、Prepared、快照及语义 MAC、适用规则和期限。活动版本／撤销头在 Reserve 和 Consume 重读。批准 ID 跨版本仅能认领一次。撤销后若需继续，须有新的人工确认及新 ID。每次预占均需批准，包含 Parent Rule 的情况。该证据证明测试 UI 的批准记录与撤销顺序，真实用户身份和知情同意仍待验证。
- 在数据库写事务内重新读取所有可变持久条件，锁住同一快照到 COMMIT。策略、规则和撤销等权威更新走同一 Writer。实际连接、进程、token、资源与 Prepared 代次来自 Core 持有的 `PreparedLease` 和 OS 核验。测试 fixture 证据须显式标识。连接断开或可信的安全字段变化使旧 lease 永久失效。事务最终核验、提交后发送前核验、Broker Launch 前核验分别设门。进程可能在最后一次 OS 核验之后退出，数据库不具备锁住 Windows 对象变化的能力。S1 只验证受控 fixture 及纯证据，实际 Peer 和资源绑定留待 S2–S4。
- 同进程的响应取用闸门与 Writer 的撤销、取消、epoch 切换串行。相关控制提交前先关闭受影响的取用槽，提交失败也不自动恢复旧响应。取用后尚未完成交付的消息按在途处理。事务和闸门均不等待 IPC 或对端确认。

## 摘要、密钥与时钟

| 名称 | 生成、验证与用途 |
|---|---|
| `decisions.input_digest` 和 `g0_reservations.operation_digest` | 保留 I05 合成输入摘要和 G0 的不可变比较。现有值在 Create 固定，与后续 HMAC 分列。 |
| `s_reservations.operation_mac` 和 `s_reservations.permit_mac` | Core 持有六域密钥。其 Writer 内部以 I06-D 按 18 组装 `operation-v2`，再以 `permit-v2` 对完整冻结票据签名。Redeem 使用明确的 key ID＋generation 和 RustCrypto 恒定时间接口分别验证两个 MAC，重算 semantic 绑定并比对事务内 Reservation。键未知、过期或撤销一律拒绝。 |
| `nonce_hash` 与请求指纹 | nonce 由 Core 在 Reserve 事务按 18 使用 BCryptGenRandom 产生，S1 只在明确的测试边界注入 fixture 值。发生碰撞，在认领和签名之前限次重生成。随机失败或连续冲突封锁该尝试及授权入口。原始 32 字节只留在短期内存和原响应中。数据库以全表唯一的 SHA-256(raw32) 关联 G0 nonce 与 S Reservation。命令指纹以闭合、typed、分域的内部数据计算，用于识别重复或冲突，无签名权。 |
| `audit-v2` | 仅 Writer 为本 activation 内经脱敏的 S 审计记录组装链接。旧段保留结构及链头，缺少历史测试密钥时旧段的 MAC 状态标为不可验证。链连续性检查与生产防截尾、防回滚是不同结论。 |

两个冻结 MAC 都直接覆盖 `reservation_id`、UTC `expires_at`、`ttl_ms` 和三种 epoch，Permit MAC 还覆盖 `operation_mac`。单调期限及其 `clock_domain_id` 通过同一不可变 Reservation 与已验 MAC、签发样本、Nonce、Peer、Prepared 绑定。兑换需要两道检查。冻结 Permit 的 MAC 对其正式字段提供密码学保证，单调字段依赖可信 Writer 和账本约束。同主体任意数据库控制下的抗篡改仍不可证。[07](../../p0.1/07-UNIFIED-EVENT-SCHEMA.md) · [15](../../p0.1/15-EXECUTION-BROKER-CONTRACT.md) · [18](../../p0.1/18-CRYPTOGRAPHIC-CONTRACT.md)

Writer 在当前事务用一个样本 `(utc_ms, mono_tick, clock_domain_id, verified_os_boot_id)` 计算两端。TTL 为显式 1–60000 ms，两个截止点均为样本加同一个 TTL。超出批准或 Prepared 任一时限，或 UTC 超过 Parent Rule 有效期，直接拒绝。G0 子步骤复用同一计算值。取样失败、回拨、溢出、未知／不一致时钟域或 OS boot 均拒绝，等于任一截止点即过期。S 路径采用严格回拨拒绝，旧 G0 时钟行为保留。单调 tick 只在一个 Writer activation 内可比。Core 或 Writer 重启换 core epoch、clock domain 和密钥世代，旧开放 Grant 失效。OS boot ID 独立于这些字段。S1 可用明确标识的注入时钟证明规则，真实取样和休眠覆盖要另验。

轮换采用关闭入口、销毁所有未送达响应并在同一 Writer 恢复事务切换 grant epoch、core epoch、时钟域及 key generation。新材料由 Core 按域生成，旧 key ID 只用于标识历史，禁止按“最新 key”兜底验证。Commit 失败或确认未知均销毁临时字节。UNKNOWN 后查询可以证明已提交的行，无法获得新签名或重新消费。

## 新增信任与待验证

新增信任假设是 Core 内部 sealed 类型和 reply slot 的所有权正确、Writer 独占句柄及 SQLite/VFS 执行按配置、Core 给出的人工批准与 Lease 证据可信、测试 Fence Marker 及启动锁由受控 harness 保持。真实跨主体边界、同 SID 抗攻击、恶意双回滚、真实 Agent 外部副作用、物理故障和真实用户知情同意均在本阶段无法证明。七款 Agent 为 L0，五项能力标志均为 false，D01–D05 待决。[待决项](I06-S-OPEN-DECISIONS.md)
