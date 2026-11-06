# G0 初始审查与基线记录

AgentGuard V2.0 · 2026-11-06 · 实现前记录

源工作区为 `D:/AgentGuard`，分支 `p1-a/first-batch`，HEAD `76c5df1adcec9196f8a27873e6ac8498873e1be1`。源目录已有未跟踪的 `docs/p1-a/NEXT-BATCH-PREFLIGHT.md`，SHA-256 `41bc91324838a19c5dbb5756a5c994939164484195caca957034fa48d5405b45`，保留原状。I05 验收工作树干净，`p1-a/i05-ledger` 本地与远端 0/0，HEAD 为 `6f9bcf50ce5b327dc0225f612b5982990fe1716a`，直接父提交为 C2 `4606de0d1c826c16aa98c6b3ac3fd15b48762899`。验收专项 Windows CI success，产物可用，摘要 `f4290b57cf20c1816eaee9c49af1984610024a6eb2adc3e19852e9ad58b8721c`。创建前确认本地/远端无同名分支且目录不存在，现已在 `D:/AgentGuard-G0` 创建独立工作树 `p1-a/g0-ledger-hardening`，起点精确为原 I05 验收提交。[I05 验收 CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42202489196)

## I05 的实际事务顺序

| 对象 | 原结构与写入 |
|---|---|
| meta、decisions、grants | meta 一行保存 epoch/归档标志。不可变 decision 含输入摘要、资格、epoch。Grant 以 UNIQUE 外键指向 decision |
| Nonce | 只保存 SHA-256 的 32 字节 hash，主键全局唯一，每 Grant 唯一。Reserve 在更新状态前插入，终态保留 |
| Reservation 截止 | 现为 Grant 双列，Rust Reserve 由写事务内可信时钟加 TTL 后一起设置。SQL 仅校验同空/非空及非负 |
| Grant 状态 | created→pending→reserved→consumed；前三态可到 revoked/expired/invalidated/failed。共 15 合法边，终态不可逆 |
| Execution/Audit | Consume 先插入 execution unknown 占位，再更新 Grant。转换后写审计；取消/Worker 失败单独更新 execution 与审计，Grant 仍 consumed |
| Writer | 临时库跨进程文件锁与单 writer，`BEGIN IMMEDIATE` 内核对 epoch、决策、nonce 和时间。状态写完后写审计，确认 COMMIT 后才给 durable 回执，未知 fenced |
| 已有触发器 | 状态触发器只在更新 state 时验证边；另有 decision、nonce 和 audit 的 UPDATE/DELETE 防护。结构、外键和领域检查用于启动及独立读取 |
| 构建 | SQLite 3.54.1，固定 rusqlite/libsqlite3-sys，官方 amalgamation，静态链接，正式 I05 编译移除扩展加载 |

已查原 migration、store/db/writer、原 19 项测试、构建配置、I05/C2 验收及 I06 预审/合同/矩阵。预审在内存复现了改绑 Grant、单独延长截止及状态无配套审计/nonce 三类缺口，也发现 recursive_triggers 关闭时 REPLACE 可绕过原 DELETE 保护。该证据只针对拥有 SQL 写权限的夹具。

## 拟采用的最小修复

新增向前 migration 003，保留 001/002 字节。保护 Grant/Decision 的 ID、关联、删除及冲突插入。比较新旧双截止的真实值，仅首次合法 Reserve 可写。新增内部 Reservation 与执行绑定证据，沿用 I05 的合成输入摘要作为精确关联；正式 semantic/operation MAC 留 I06。使用延迟外键把当前状态及新 Reserved/Consumed 配套记录与同 Grant 审计绑定，保留先写状态后写审计的语义。受控 writer 在 COMMIT 前复核完整领域关系，SQLite 延迟约束同时检查原始写连接。启动先按旧版模板校验再迁移，保留已消费历史，旧未消费失效，记录切换点。正式数据库不接受缺失约束的数据，异常留原始证据。未来 V 测试和 T2-B 整体篡改结论继续留 D01/D05。

[原 SQL](../../crates/ledger/migrations/001-grants.sql) · [原 I05 报告](I05-LEDGER-REPORT.md) · [C2 报告](I05-C2-ACCEPTANCE.md)
