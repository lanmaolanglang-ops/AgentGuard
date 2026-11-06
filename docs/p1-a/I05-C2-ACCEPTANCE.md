# I05 C2 · SQLite 固定构建与临时数据库验证

AgentGuard V2.0 · 2026-11-06 · 分支 `p1-a/i05-c2`

**本机 C2 技术门槛 PASS。** 本记录提交给负责人复核。范围为 Windows x64 上的来源核验、WAL/FULL、跨进程写者竞争、进程强退重开和页限故障。账本主体等待复核。

## 固定构建与修复证据

| 项目 | 观测 |
|---|---|
| 构建 | Rust/Cargo 1.97.1，`x86_64-pc-windows-msvc`，`rusqlite 0.41.1` + `libsqlite3-sys 0.39.1` bundled，锁定 11 个 registry 包。复算全部 crate 档案 SHA-256，与独立 Cargo.lock 一致 |
| SQLite 源码 | crate 内 `sqlite3.c` SHA3-256 为 `e2b869d3003b33c942b7dd7063604fbc53553e1b66609570df39b9b7b558ee81`，与官方 3.54.1 一致 |
| 实际运行时 | `sqlite_version()=3.54.1`，`sqlite_source_id()=2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76` |
| 链接及构建脚本 | 新临时 target 冷构建，Cargo 输出 `static=sqlite3`。核对 build.rs，bundled 路径编译固定 C 源并复制 bindings，禁止 pkg-config、系统库及编译参数覆盖。已核脚本 SHA-256 为 `769b31370a41aa722e1d5a530012432c985bb142eab34d095481ffd1a3436e1b` |
| 修复状态 | 3.54.1 包含 WAL-reset 修复，且包含对 3.54.0 的上游补丁。2026-11-06 用 OSV querybatch 查询 11 个锁定 crate 版本，返回 0 命中。该查询只反映数据库可见公告 |
| 运行参数 | 本机 Windows 10.0.26200，`COMPILER=msvc-1944`，`THREADSAFE=1`，`journal_mode=wal`，`synchronous=2`（FULL），`foreign_keys=1`。探针在故障期间禁用自动 checkpoint 以留下可检查 WAL |

[SQLite 3.54.1 发布与源码哈希](https://sqlite.org/releaselog/3_54_1.html) · [WAL-reset 修复](https://sqlite.org/wal.html#the_wal_reset_bug) · [Rust 绑定](https://docs.rs/crate/libsqlite3-sys/0.39.1) · [锁文件](../../probes/i05-c2/Cargo.lock)

## 进程故障与独立持久化取证

| 检查 | 前提与实际观测 | C2 结果 |
|---|---|---|
| 临时目录限制 | harness 在本机固定盘建立一次性目录。无 fixture 标记的独立临时目录被拒绝，目录保持空 | PASS |
| 单写者 | 持有者未退出时，第二探针进程申请同一文件锁得到 busy。绕过应用锁的故障专用进程执行 `BEGIN IMMEDIATE` 得到 `SQLITE_BUSY=5`，150 ms SQLite 超时，外部子进程耗时在验证预算内 | PASS |
| 真实异常退出 | 持有者先以 FULL 提交 2 行，再打开新事务写入 64 行并让缓存溢写 WAL。独立进程读到 2 行。harness 终止持有者，确认退出非零且重开前 WAL SHA-256 未改变 | PASS |
| WAL 取证 | 本机提交后 WAL 12,392 B，未提交溢写后 572,712 B。独立解析观察到 139 帧，最后 commit 标记在第 3 帧，崩溃后原主库仍 4,096 B | PASS |
| 崩溃重开 | 新进程重新获得 writer 文件锁并开库，随后独立只读进程重查。只剩 `1|committed|01` 和 `2|committed|02`，`rows=2`、`integrity=ok`，64 行未提交数据未恢复 | PASS |
| 写入失败 | 限制临时库页数，在事务内先插入一行，再插大 blob 触发 `SQLITE_FULL=13`。独立只读进程复查仍为同两行和 `integrity=ok` | PASS |

该 probe 只操作固定测试行。初次编译发现 SQL INTEGER 读取类型不匹配，改用 SQLite 对应的 `i64`。首轮故障取证发现持有者的计数查询继续触发三帧溢写，随后在查询完成后采样，保留 WAL 长度与摘要必须相符的验收。故障模型是 Windows 进程强制终止和 SQLite 页数限额。证据不覆盖断电、物理满盘、真实授权账本、生产数据恢复或恶意整体回滚。

## 代码与执行入口

- [独立 C2 Rust 探针](../../probes/i05-c2/src/main.rs) 固定 SQL、临时文件作用域、文件锁、只读查询和故障注入。
- [C2 编排脚本](../../probes/i05-c2/run.py) 校验来源、冷构建、启动和回收子进程，输出实际摘要及独立读库结果。
- [Windows C2 工作流](../../.github/workflows/p1-a-i05-c2.yml) 固定工具链，运行同一入口、Rustfmt 和 Clippy，上传合成 JSON 证据。

~~~powershell
python -B probes/i05-c2/run.py
~~~

探针接收固定操作名，拒绝外部路径和 SQL。连接未启用扩展加载，当前 bundled 编译含相关代码。此构建参数只用于 C2 验证，未来账本选型和部署仍需审查。原第一批代码、锁文件和验收证据保持不变。I05 正式 AT21/22/29/36/45 尚未执行，C2 只提供其中涉及 SQLite 机制的先验证据。七款 Agent 均 L0，五项标志均为 `false`。D01–D05 待决。

[C1 共享契约](I04-I05-C1-CONTRACT.md) · [正式计划](../p0.1/14-P1-DEVELOPMENT-PLAN.md)
