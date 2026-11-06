# I04 合成分类验收

AgentGuard V2.0 · 2026-11-06 · 分支 `p1-a/i04-synthetic`

I04 交付一个独立无第三方依赖的 Rust 包。唯一入口接收代码中登记的 `FixtureId`。分类器使用夹具提供的结构化证据，按硬禁、原生拒绝、证据缺失和操作分类处理。输出携带风险、原因、处置、影响、目标及缺证据，关联精确输入与 epoch。对 raw shell、未知工具及不可验证子命令保持保守。模型或 UI 自述不会进入策略判定。正向 `SyntheticCandidate` 仅表示合成分类条件匹配。

## 代码与验收范围

| 新增文件 | 职责 |
|---|---|
| [Cargo.toml](../../crates/policy/Cargo.toml)、[Cargo.lock](../../crates/policy/Cargo.lock)、[.gitignore](../../crates/policy/.gitignore) | 独立构建、无新增依赖、局部构建产物忽略 |
| [lib.rs](../../crates/policy/src/lib.rs) | 私有类型与分类器、闭合 fixture 入口、关联与结构化输出 |
| [fixtures.rs](../../crates/policy/src/fixtures.rs) | 17 个编译期合成输入 |
| [tests.rs](../../crates/policy/src/tests.rs) | 9 个 AT 映射及 1 个确定性测试 |
| [Windows CI](../../.github/workflows/p1-a-i04.yml) | 固定 Rust 1.97.1，同入口执行格式、Clippy 和 AT 测试 |

## 测试记录

本机 Windows x64/MSVC、Rust 1.97.1。Rustfmt、Clippy `-D warnings`、Cargo test 通过。测试函数 10 通过、0 失败、0 忽略。17 夹具与 23 个独立字段突变属于数据覆盖。

| AT 合成子范围 | 可复现断言 | 本地结果 |
|---|---|---|
| AT01 | 有身份、对象和根证据的只读候选。缺证据进入复核，越界拒绝 | PASS |
| AT02 | 未获额度的破坏性操作拒绝。假 UI 批准与身份变化无法覆盖硬禁 | PASS |
| AT03 | 管道与未知工具的副作用保持 unknown | PASS |
| AT04 | 编码、动态求值等惰性文本不成为候选，结果不回显命令 | PASS |
| AT05 | 固定嵌套描述保留层次，已知保护目标拒绝，未知内层保持 unknown | PASS |
| AT06 | 脚本变化使旧关联失配，Python 子进程效果未知 | PASS |
| AT07 | npm 生命周期被判高风险，正向只读对照仍为低风险 | PASS |
| AT19 | 逐一变更 23 个输入或 epoch，均不能匹配旧关联 | PASS |
| AT26 | 模型/MCP 文本无授权权能，硬禁与原生拒绝保持，未知进入复核 | PASS |
| 确定性 | 17 个输入逆序复跑输出相等，唯一只读合成候选 | PASS |

~~~powershell
cargo +1.97.1 fmt --manifest-path crates/policy/Cargo.toml -- --check
cargo +1.97.1 clippy --manifest-path crates/policy/Cargo.toml --locked --all-targets -- -D warnings
cargo +1.97.1 test --manifest-path crates/policy/Cargo.toml --locked
~~~

## 安全与阶段边界

分类器无文件、进程、网络、环境解析或 SQLite 接口，scope、嵌套语义和对象证据仅来自预先登记的合成输入。它既不解析任意 Windows Shell，也不解析真实路径或判断真实影响。结果模型无签发、兑换或执行字段。代码没有接到 I03 的 `evaluate` 或 IPC。真实参数最终绑定、撤销与执行证据分别留给后续节点。七款 Agent 均 L0，五项标志均为 `false`。第一批代码和原设计原样继承。

[C1 共享契约](I04-I05-C1-CONTRACT.md) · [冻结 AT 合同](../p0.1/10-TEST-AND-ACCEPTANCE-PLAN.md)
