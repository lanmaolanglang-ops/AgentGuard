# I05 主体账本工程交付

AgentGuard V2.0 · 2026-11-06（Asia/Shanghai） · `p1-a/i05-ledger`

**工程门禁通过，待负责人独立复核。** 本分支以已验收的 C2 提交 `4606de0d1c826c16aa98c6b3ac3fd15b48762899` 为父基线，第一批与 C2 原文件原样继承。账本入口限带标记的临时夹具。它接收进程内可信测试输入，返回持久化状态，当前 Core、I03 IPC 和 I04 分支保持独立。最终验收由负责人作出。

## 变更清单

| 文件 | 职责 |
|---|---|
| [Cargo.toml](../../crates/ledger/Cargo.toml)、[Cargo.lock](../../crates/ledger/Cargo.lock)、[.cargo/config.toml](../../crates/ledger/.cargo/config.toml)、[.gitignore](../../crates/ledger/.gitignore) | 独立构建、SQLite/摘要/epoch 依赖锁定、移除扩展加载、忽略局部构建产物 |
| [001-grants.sql](../../crates/ledger/migrations/001-grants.sql)、[002-events.sql](../../crates/ledger/migrations/002-events.sql) | 账本表、索引、外键、触发器、事件去重 |
| [lib.rs](../../crates/ledger/src/lib.rs)、[db.rs](../../crates/ledger/src/db.rs) | 内部强类型、存储与 schema 校验、只读对账、备份归档 |
| [store.rs](../../crates/ledger/src/store.rs)、[writer.rs](../../crates/ledger/src/writer.rs) | 唯一 writer、事务和状态机、有界优先队列、持久化确认 |
| [tests.rs](../../crates/ledger/src/tests.rs)、[verify_i05_ledger.py](../../tools/verify_i05_ledger.py)、[Windows 工作流](../../.github/workflows/p1-a-i05-ledger.yml) | AT、独立进程故障注入和读库、固定构建与证据打包 |

## 数据库与迁移

| 表 | 关键约束 |
|---|---|
| `meta` | 单行，Core boot、policy/grant epoch、归档标志 |
| `decisions` | 不可变，决策 ID、输入摘要、epoch 和内部合成资格 |
| `grants` | 每决策最多一个 Grant，状态与双时钟期限。SQL 触发器拒绝非法迁移 |
| `nonces` | SHA-256 nonce hash 为全库主键，Grant 唯一外键。终态与重启后保留 tombstone |
| `audit` | 追加式序列，关联 Grant/决策、前后状态及固定审计类型。唯一请求 ID 与请求指纹支持仅状态对账 |
| `execution` | 仅关联 consumed，记录 unknown 或 synthetic worker failed 与取消请求。执行状态独立于授权 |
| `observations` | source/event ID、source sequence 双唯一约束及内容摘要。独立 ingest 序列，晚到事件可填补缺口且不能改授权 |

数据库使用本地 SQLite WAL、FULL、外键和 150 ms busy 上限。私有 `application_id` 和 `user_version` 与冻结的 wire Schema 版本分别维护。迁移 001 创建授权与审计表，002 增加观察事件。启动在同一事务中执行 1→2 迁移、结构/外键/领域一致性检查和旧未决 Grant 失效。拒绝未知版本、损坏、schema 漂移及同一活动库上的数值 epoch 降低。检查将实际 `sqlite_schema` 与两份迁移生成的内存模板逐项比较，审计链、末状态、nonce 关联、取消与执行记录也交叉验证。

## 写入、回执与恢复

- **串行入口。** 临时目录内固定文件锁跨进程阻止第二个 writer。单线程持有写连接，控制队列 128 和观察队列 64 各自限额。观察洪水无法占满控制队列，当前事务结束后优先处理控制。独立取证连接使用只读 SQLite 标志及 `query_only`。
- **事务关联。** `BEGIN IMMEDIATE` 后核对数据库当前 policy/grant/Core epoch、决策 ID、不可变输入摘要和资格。Reserve 登记唯一 nonce、双时钟截止、状态及 audit；Consume 核验同一关联与 nonce，条件更新 reserved、写 unknown 执行记录与审计。三个写入都在各自事务内完成。正式 MAC、签票与最终执行绑定属于后续合同。
- **持久化确认。** 成功 COMMIT 后才返回 `Outcome::Durable(Receipt)`。返回的序列是对应审计记录，字段无法表达启动权限。同请求同指纹只返回 `Reconciled`，同 ID 异内容拒绝。当前 epoch 内被拒绝且无持久回执的请求 ID 留在有界集合，条件随后满足也不能用同 ID 变成写入。集合到 4096 项或遇到 busy、提交错误、提交后丢响应及数据库错误时停止写入，调用者用独立读库对账，重开使旧 Grant 失效。
- **顺序与失效。** 撤销先提交则消费无法改变 revoked；消费先提交则 consumed 不变，可单独持久化取消请求。Worker 失败只改变执行记录。TTL 上限 60 秒，写事务中按墙钟高水位与进程单调时间较早到期，等于截止即失效。Created/Pending 准备到期由可信内部动作标记，Reserved 必须复查时钟。重开生成新 boot epoch，把旧 created/pending/reserved 变为 invalidated，保留已消费历史及 unknown。
- **旧备份。** Online Backup API 写独立临时目录的 staging 文件。归档标志写入并检查后发布，只读接口打开历史。把归档文件改名为活动库仍拒绝；活动库与现行授权不会被旧副本替换。历史视图可能显示备份时为 reserved，当前权限以活动账本和 epoch 为准。

[冻结状态机与事务](../p0.1/08-SECURITY-POLICY-ENGINE.md) · [故障恢复合同](../p0.1/15-EXECUTION-BROKER-CONTRACT.md) · [C1 接口边界](I04-I05-C1-CONTRACT.md)

## 测试执行与独立取证

本地 Windows x64、Rust/Cargo 1.97.1 冷构建执行 19 个测试，19 通过、0 失败。另有一个子进程驱动用例由父测试按指定故障点调用，不单独计入通过数。Rustfmt 和 Clippy `-D warnings` 通过。专项 CI 同入口重新构建并归档输出。下表为本批实现的 AT 范围。

| AT | 独立观察与结果 |
|---|---|
| AT21 | 重复与重连/重启去重、内容/来源序列冲突、不同来源同号、晚到填补。四条持久 ingest 顺序，gap 2→1→0，晚到未改授权 |
| AT22 | 100 路同时预占仅 1 次提交，100 路同 nonce 消费仅 1 次提交、99 次重放拒绝。独立读库只有一个 consumed、一个 nonce、四条转换，测试 sink 只出现一次。重复请求只对账，重启后全局 nonce 仍不能复用 |
| AT29 | 两个队列溢出拒绝，控制可在观察队列饱和时入队。独立进程竞争观察到文件锁 busy 和 SQLite busy。页数限额触发 SQLite 13，审计写触发器故障为 1811。未返回 ACK，重开前读库仍为 reserved、无消费/执行记录。I03 帧与畸形协议由第一批原工作流回归 |
| AT36 | 注入墙钟后退、单调截止、前跳后再回拨、准备到期和真实进程重启。旧 epoch 拒绝，已消费保留，已知旧归档保持只读。模拟时间验证与实际进程重开分别记录 |
| AT45 | 64 个状态对在 SQLite 上核验，15 合法、49 拒绝。覆盖预占、消费、审计提交、响应、模拟启动故障点，真实 COMMIT 外键故障、提交结果未知对账、撤销两种提交顺序及同时竞争、合成 Worker 非零退出。独立进程查询持久状态与审计序列 |

| 跨进程强退点 | 强退后、重启失效前的持久事实 | 重启后 |
|---|---|---|
| Reserve 写状态后、审计前 | pending，2 转换，nonce 0 | invalidated |
| Reserve 写审计后、提交前 | pending，2 转换，nonce 0 | invalidated |
| Consume 写状态后、审计前 | reserved，3 转换，nonce 1 | invalidated，保留 nonce |
| Consume 写审计后、提交前 | reserved，3 转换，nonce 1 | invalidated，保留 nonce |
| Consume 提交后、响应前 | consumed，4 转换，execution unknown，sink 未触发 | 保留 consumed，仅对账 |
| 合成 Launch 标记前 | consumed，execution unknown，sink 未触发 | 保留 consumed，仅对账 |
| 合成 Launch 标记后 | consumed，execution unknown，sink 存在一次 | 保留 consumed，重试不再触发 sink |

测试驱动用自己的 Windows 子进程和临时磁盘文件。进程强退前确认已达故障点，强退后记录 WAL 大小，再由另一个进程只读查询并核对 Grant/Nonce/Audit/Execution，重启后复查。提交故障用延迟外键使实际 COMMIT 失败，确认无 ACK、无消费回执。另在提交完成后模拟响应故障，确认只可查询已持久回执。Worker 是固定的非零退出夹具，Launch sink 是测试文件标记。

~~~powershell
python -B tools/verify_i05_ledger.py
~~~

本机输出在 `.work/i05-ledger/at-results.log` 与 `evidence.json`。CI 产物包含同名证据。

## SQLite 供应链与安全边界

锁定 `rusqlite 0.41.1`、`libsqlite3-sys 0.39.1`，捆绑官方 SQLite 3.54.1。源码 SHA3-256 为 `e2b869d3003b33c942b7dd7063604fbc53553e1b66609570df39b9b7b558ee81`。实际 source ID 为 `2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76`。冷构建复算 24 个 registry 档案并检查 SQLite 构建脚本与 `static=sqlite3` 链接输出。2026-11-06 OSV querybatch 对 24 个锁定 crate 版本返回零命中，反映查询当时的已知公告。SQLite 版本包含 WAL-reset 修复。[SQLite 发布及源码哈希](https://sqlite.org/releaselog/3_54_1.html) · [WAL-reset 说明](https://sqlite.org/wal.html#the_wal_reset_bug)

正式构建使用 `-USQLITE_ENABLE_LOAD_EXTENSION -DSQLITE_OMIT_LOAD_EXTENSION`。运行时要求 `OMIT_LOAD_EXTENSION=1`、`ENABLE_LOAD_EXTENSION=0` 及固定 Source ID，专项 CI 同时实测 `load_extension()` 函数不存在。C2 原探针仍保留自己的构建与验收记录。

独立测试证据限于临时库、应用级单 writer 和本地进程崩溃模型。物理掉电、真实设备满盘、外部执行或网络副作用、跨安全主体攻击和恶意整体回滚仍待对应节点。文件锁与本地审计完整性检查的信任范围限于受控测试环境。审计 MAC、真实原生批准、签发、Broker、OS 隔离和生产恢复尚未启用。D01–D05 待决，七款 Agent 均为 L0，五项能力均 `false`。等待负责人独立复核 I05。
