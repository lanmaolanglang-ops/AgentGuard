# I06-S S1 实现与核验入口

本候选实现 [负责人批准的 S1](I06-S-S0-OWNER-APPROVAL.md)。S0 的接口、时序、迁移和故障矩阵保留为审批时的设计记录。四个验收 SHA 以及 42 份冻结设计文件由验证入口核对。

| 实际变更 | 路径与行为 |
|---|---|
| I06-D 预提交校验 | `crates/typed-authorization/src/s_preflight.rs` 严格解析冻结结构，验证合成 recipe、身份和 Prepared 形状，生成不可变候选。Writer 用当前密钥、时钟和已登记事实重核快照，组装 `operation-v2` 和 `permit-v2`。原 I06-D 内存模型继续独立工作。 |
| G0 Writer | `crates/ledger/src/writer.rs` 增加封闭 `SCommand` 和一次性取用。S 与 G0 共用队列、写连接、`BEGIN IMMEDIATE`、原状态转移及审计约束。G0 入口对 S Grant 和跨命名空间请求冲突拒绝。 |
| v4 模式 | `crates/ledger/migrations/004-s-atomic.sql` 追加 STRICT 表、约束、触发器、审计及跨表核验视图。`Ledger::create_s` 新建合成 fixture，`migrate_s_copy` 只读打开 v3 源并使用 SQLite backup 建立独立副本。旧创建和重开路径保留版本 3。 |
| Reserve 与 Consume | `crates/ledger/src/store/s_transaction.rs` 处理预登记政策、人工批准和规则的活动版本。Reserve 在签名前对 nonce 做最多两次生成，认领、扣额、G0/S 预占、审计和收据同提交。Consume 最终验证两个 MAC 和双时限，将启动登记、`execution=unknown`、审计及收据同提交。 |
| 一次性响应 | 提交确认后 Writer 产生 `DurableCommitReceipt`。原请求持有不可复制的取用凭据。`Client::take_s` 经同一队列检查 Peer、时限、撤销和 Fence，取走内存中唯一响应。丢失、过期或关闭的回复只留下持久化状态。撤销与取用在队列中有确定顺序，已取出的响应按在途处理。 |
| Reconcile | `Inspection::reconcile_s` 使用独立只读连接、单一快照和指定 Grant 作用域，返回 `Committed`、`NotObserved`、`Unavailable` 或 `Unknown`。公开视图只含历史标识、状态及执行未知事实。 |
| Fence 与恢复 | `crates/ledger/src/store/s.rs` 持有 G0 fixture 锁及外置 lineage 锁。正常恢复要求 Marker、数据库头、完整性一致，增加 activation、grant epoch 和密钥世代，更新 Core epoch 和时钟域，失效旧开放 Grant 并保留消费和审计。旧 v4 归档可只读检查。Marker 与数据库分叉时保持关闭供调查。 |

合成 Parent Rule 注册显式提交 `3` 次和 `600000` ms，绑定 Writer 观察到的批准 UTC 时间，跨规则版本保留历史用量。新规则版本要求新的合成批准事件。OneShot 与 Parent 路径都须认领一份单次批准。每份 Permit 的 TTL 仍为 `1..=60000` ms。

故障测试代码位于 `crates/ledger/src/store/s_tests.rs`。独立的 `tools/inspect_i06_s.py` 使用 SQLite URI `mode=ro` 和 `query_only`，核验 SQLite 完整性、外键、额度与对应行，并保存同快照副本及脱敏事实。每项的运行状态以验收入口生成的 `matrix.md` 和 `summary.json` 为准。

在 Windows x64 使用固定 Rust 1.97.1、Python 3.14.6、Node 24.19.0 和 `requirements-ci.lock` 执行。

```powershell
python -B tools/verify_i06_s.py --baseline-worktree D:\AgentGuard-I06-D
```

省略 `--baseline-worktree` 时，脚本从验收 SHA 建立独立临时工作树。脚本在 I06-D 验收 SHA 运行原验收脚本，再对候选运行原有测试、C2、第一批测试、冻结设计校验、F01–F24、Schema 对照，以及用验收版 G0 代码编译的 v3/v4 回退探针。证据 ZIP 与外部 SHA-256 文件写入 `.work/i06-s/verified`，包内另含逐文件 `SHA256SUMS`。

`.github/workflows/p1-a-i06-s.yml` 在 Windows 2022 使用同一入口。远程工作流结论以该候选 SHA 对应的 GitHub Actions 结果为准。发布候选遵循负责人审查后的推送门禁。

S1 的证据范围限于合成 fixture、数据级一次性和受控恢复。审计仅核验当前 activation 的 MAC，历史段核验结构和链接元数据。测试 Marker 不覆盖同主体同时回滚数据库与 Marker。真实 Agent、真实 Peer／休眠／跨进程时钟、真实文件读取、真实 Broker／Worker、副作用单次性、生产密钥、生产恢复、物理掉电及 Windows 系统隔离均超出该证据范围。F25–F28 留在后续阶段。
