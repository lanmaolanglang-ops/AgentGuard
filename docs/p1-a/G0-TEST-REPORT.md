# G0 测试与持久化取证报告

AgentGuard V2.0 · 2026-11-06 · 合成工程结果

## 执行入口

~~~powershell
python -B tools/verify_g0.py
~~~

入口依次核验正式 ledger、C2 探针、第一批协议。任一核心阶段失败时保留已执行日志并停止后续门禁。CI 使用相同入口。Rustfmt、Clippy 和 locked x64/MSVC 冷构建纳入 ledger 门禁。

| 项目 | 本地实际结果 |
|---|---|
| 正式 SQLite | 3.54.1、Source ID、上游源码 SHA3-256 与本地 SHA-256、静态链接、扩展加载编译移除，实际读回 WAL/FULL/FK、recursive=1、trusted_schema=0、busy=150 ms、autocheckpoint=1000 |
| I05 | 原 19 测试通过。100 路预占与消费各一次，7 个强退点，撤销、Worker 失败、SQLITE_FULL、Fenced 和独立重开通过 |
| G0 | 新增 8 个函数，36 个指定编号和两个递归触发器配置均覆盖。账本总计 27 通过、0 失败。额外一个子进程驱动 marked ignored，由父测试以精确名称执行，独立读库与故障行为均在日志有证据 |
| 状态对 | 64 组，15 合法、49 拒绝。正向使用真实配套 SQL 与领域检查 |
| C2 | 原探针通过 WAL/FULL、单写者、SQLite BUSY、独立强退重开及页限 FULL。只恢复 2 条已提交行，64 条未提交行未恢复。64 状态对的证据来自 I05 正式库 |
| 第一批 | 原 10 项门禁，44 项通过、0 失败，42 份基线哈希一致 |

## 独立证据细分

1. **修复前对照。** 在相同静态构建，用冻结 001/002 的 v2 临时库复现历史 FK 改绑、截止延长及缺执行/审计的 consumed。独立连接读取对照数据，领域检查拒绝。
2. **修复后语句拒绝。** 每次攻击前后用新只读连接获取逻辑表快照并哈希，校验 FK、域关系。G0a 的两组配置独立记录，冲突或删除等操作无部分数据残留。
3. **修复后提交拒绝。** G0c-04、G0c-06 的单条写入成功，实际 COMMIT 返回 FK 扩展错误 787，快照不变。Writer 故障注入还分别证明真实 COMMIT 错误无回执，提交后丢响应保留唯一记录及 fenced，独立查询只对账。
4. **故障及迁移。** 强退后由另一进程读取 Grant、nonce、audit、execution 与双绑定计数，恢复前后和 marker 分开。旧 reserved 失效，consumed/nonce 保留，不创建旧 binding。损坏 v2 拒绝，字节 SHA-256 保留。
5. **时间。** UTC 截止前一次正常消费，等于及超过 UTC/单调截止时 expired、无 Execution。原时钟回拨、前跳后回拨、Core 重启及旧 epoch 拒绝回归通过。

日志中的测试运行成功、SQL COMMIT 持久成功及文件 marker 分开核查。本轮真实进程强退与 SQLite 页限使用专用夹具，分别和真实断电、物理满盘、Agent 执行区分。[验收矩阵](G0-ACCEPTANCE-MATRIX.md) · [CI 与证据目录](G0-CI-EVIDENCE.md)
