# I06-S 事务原子性故障矩阵

**全部为计划用例，当前 NOT VERIFIED。** S1 使用一次性合成库、可控时钟和独立只读检查进程。涉及真实进程链和 Peer 的用例留待相应后续阶段。现有 G0 证据只支持其原事务和边界。[事务时序](I06-S-TRANSACTION-SEQUENCE.md) · [迁移设计](I06-S-MIGRATION-DESIGN.md)

## 对照及独立观测

对照 A 从同一份受信快照开始，合成人工批准后 Reserve 产生一个耐久回执，查询见一个认领、一次扣额、一个 nonce、两个关联的 Reservation 记录及完整审计。仅原请求得到一份 Permit。

对照 B 在 A 后 Consume，查询见 Grant consumed、G0 execution unknown、一个 launch ID、完整审计及收据。仅原请求得到一个启动响应，合成接收器记录一次使用。需要验证实际执行时，由独立 Observer 记录结果和实际进程状态。

每个故障运行采用同配置的新 fixture，先跑相应对照，再注入。独立进程在单个只读快照记录原 Grant、Approval claim、Rule 用量、G0/S Reservation、nonce、Execution、Launch 登记、请求收据、审计、Fence 与完整性结果。用脱敏客户端记录核对响应类型和数量。绝不记录原 nonce、密钥、Permit 或启动响应正文。对崩溃在进程外终止并重新打开，记录失败边界、进程退出证据和恢复后的实际状态。

## S1 数据级与 Writer 故障

| ID | 对照与注入点 | 预期响应和落库状态 | 独立证据 |
|---|---|---|---|
| F01 | A，缺批准、替换批准版本、伪造人工来源、跨 Grant 复用 ID | 拒绝。零 claim、零扣额、无本次 Reservation、无本次状态转移 | UI 作用域证据、批准头与四表差分 |
| F02 | A，Parent Rule 版本已被替换、已撤销或失效，尝试降格为 OneShot | 全拒绝。原 scope 与计数保持 | 规则／批准头、事务快照和审计序号 |
| F03 | A，100 个已明确批准请求争用最后一份额度 | 恰好一组成功，其他拒绝。用量增加一，claim／nonce／Reservation／审计均与胜者对应 | 100 个请求结果及唯一行关联 |
| F04 | A，随机源先碰撞后提供新 nonce，再注入持续碰撞、重复 Reservation ID、不同请求指纹使用旧 ID | 单次碰撞的成功组只登记新 nonce。连续冲突或 ID 冲突拒绝。失败路径无扣额、无新 claim 或收据 | 注入计数、唯一索引、原收据和额度前后对照 |
| F05 | A，在候选入队后撤销批准、变更 policy、规则或 Prepared | Writer 事务重读拒绝。旧 prepared lease 不可恢复，缓存命中无效 | 提交顺序、旧 lease 与当前记录，过期候选的响应 |
| F06 | A，Reserve 已写 claim 或扣额后，在 nonce、G0/S 预占、状态、审计或收据任一步失败 | 只要回滚确认，整组缺席，原 pending 保持。无 Permit | 对每个写点独立读回，包含审计 FK 失败 |
| F07 | A，Reserve COMMIT 前强制终止 | 外部可能 UNKNOWN。恢复后无本次提交组，旧开放 Grant 因新 activation 失效 | 外部终止证据、前后 epoch、缺失的提交组与恢复审计 |
| F08 | A，Reserve COMMIT 后、响应前终止，或明确丢弃响应 | 查询见完整 Reserve 组和一次扣额。旧 epoch 下预占已失效或被 fence。无补发 | 原请求查询、状态／审计、发送计数为零 |
| F09 | A/B，在 `BEGIN IMMEDIATE`／写阶段注入 BUSY、FULL，另在 COMMIT 注入失败 | 回滚已确认返回拒绝。提交不明返回 UNKNOWN 且封锁。无权利响应 | SQLite 错误阶段、独立整组检查、fence |
| F10 | A，COMMIT 已成功但调用端排队超时或断线 | 原 reply slot 被关闭，晚回复销毁。若已提交仅状态可见 | 队列时间线、收据、零补发 |
| F11 | B，签名错误、跨域／旧 key、有效签名但错 Reservation／Peer／generation／快照 | 全拒绝，无消费和启动登记。记录明确拒绝类别 | 测试独立构造正反载荷，校验原 reserved 保持 |
| F12 | B，UTC 到期／mono 到期分别测试，TTL 在队列或 COMMIT 内耗尽，或时间回拨、溢出、缺失样本 | 到期或异常不发权利响应。若已经提交只查询。旧时钟域值不参与比较 | 注入样本、两个期限、提交与发送时间线 |
| F13 | B，100 个同 nonce 并发兑换，包含同 request ID 和不同 ID | 一次消费、一条启动登记、一份原响应。重复同指纹只读，其他请求拒绝 | 独立统计消费／启动行、审计及 100 个客户端结果 |
| F14 | B，Consume 写入 execution 或启动登记后，在审计／收据写入处失败 | 回滚后保持 reserved，execution／launch／收据均缺席，无启动响应 | 状态与相关表一致性读回 |
| F15 | B，Consume COMMIT 前强制终止 | 查询不能从无收据推断发送。无本次消费、执行与启动登记，恢复失效旧开放 Grant | 强退证据、同快照检查与恢复审计 |
| F16 | B，故障注入点确认 Consume COMMIT 成功，随后在发送前终止或丢弃响应 | consumed、launch 登记和审计均存在。调用方可能 UNKNOWN，Execution 仍 UNKNOWN。查询无重发、重扣或自动启动 | 同请求只读查询、重复提交以及接收器零启动 |
| F17 | A/B，撤销先于消费，再以相反顺序做对照 | 先撤销则拒消费。先消费则保持 consumed，后续写在途取消事实 | 单 Writer 审计序号、一次消费计数和取消记录 |
| F18 | A/B，旧 G0 入口尝试写 S Grant，并用非 S Grant 验旧入口 | S 路径拒绝旁路，非 S 调用保持旧语义 | S 归属、G0 与 S audit／receipt、原有回归 |
| F19 | A/B，Core 或 Writer 重启，或密钥轮换 | 新 activation 禁旧票，旧开放 Grant 失效，已消费终态及 nonce 历史保留。新请求须新的人工批准 | 两个 epoch／clock domain、只读旧收据、重放拒绝 |
| F20 | A/B，系统重启、OS boot／Writer clock domain 不同或不可证 | 禁止复用旧 ticks 及旧许可。未经核验关闭入口 | 两个时钟域和 boot 身份证据。S1 只以注入值验证规则 |
| F21 | A/B，Marker 已推进但 DB 未提交、DB 已提交但尚未开放、Marker 丢失／被替换 | 不匹配或未知时关闭。匹配后仍需重新核验并清除旧发送槽。启动从无旧响应 | 独立 Marker、DB 激活头与启动门禁事件 |
| F22 | A/B，恢复较旧备份、提交旧 Reservation 或尝试同 lineage 双开 | 历史只读或拒绝，旧许可无效，原记录保留 | Marker 与归档身份、唯一 Writer 锁、源库和归档哈希 |
| F23 | v3 对照，追加迁移中断、Schema drift、迁移提交确认丢失及程序回退 | 事务整体回滚或 fence 待查。旧程序拒绝 v4，v4 历史仍可只读取证 | schema、版本、完整性、源 v3 字节、历史记录与回退日志 |
| F24 | A/B，对账访问跨作用域、读旧快照无记录、伪造收据或缺审计 | 无任何权利响应。返回权限错误、NotObserved、Unavailable 或保留 UNKNOWN | 读侧仅运行查询、独立快照核验、响应结构检查 |

## 后续进程链故障

| ID | 对照与注入点 | 预期状态 | 独立证据 |
|---|---|---|---|
| F25 | A，Reserve 后 Broker 崩溃，旧连接重连和伪造 Peer | 无旧 Permit 补发，旧连接无法兑换 | OS 来源进程实例、连接序号及查询记录 |
| F26 | B，Launch 响应送达但 Worker 尚未开始，重复发送或重启 Broker／Worker | 单次门禁拒绝重复。证据缺失时执行 UNKNOWN。新进程无自动启动 | 独立进程退出／存活、启动 sentinel 和接收端单次计数 |
| F27 | B，Worker 已启动但 Observer 未记录或断线，伪造 Worker 成功 | Worker 自报与独立证据分离。未获独立证据保持 UNKNOWN／未证实 | 观察时序、固定 fixture 摘要和独立进程证据 |
| F28 | B，Cancel 请求及 ACK 已送达，Worker 未确认停止 | 仅记录收到取消，停止未证实。Grant 保持 consumed | ACK 与进程终止证据分别取证 |

覆盖 AT17–19、22、30、36、39、44、45 的合成子项沿用既有定义及 [S0 映射](I06-S-IMPLEMENTATION-PLAN.md)。真实 Agent、真实 OS 隔离、生产可信时间和物理掉电场景保持 NOT VERIFIED。接受时按用例填预期、实际、测试环境、证据位置、提交和结果，禁止以矩阵数量代替运行证据。
