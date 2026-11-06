# I06-D 工程交付

实施分支 `p1-a/i06-d-typed-contract` 以负责人验收的 G0 提交 `8c2a026468e05b352e4e3ed9de1db3dcc3ce8743` 为起点。交付后停在负责人独立复核门禁。I06-S、I07-A/B、I22 和真实 Agent 接入需要各自批准。

| 新增路径 | 交付内容 |
|---|---|
| `crates/typed-authorization/Cargo.toml`、`Cargo.lock` | 独立 workspace，复用原固定依赖。 |
| `crates/typed-authorization/src/lib.rs`、`model.rs`、`clock.rs` | 纯数据接口、角色、三种 epoch、时钟和状态结果。 |
| `crates/typed-authorization/src/state.rs`、`snapshot.rs` | Parent Rule、Prepared、Grant、Reservation、一次合成 Permit、对账与失效。 |
| `crates/typed-authorization/src/crypto.rs`、`tests.rs`、`fixture-macs.json` | 六域 payload、MAC 验证、正反例和公共合成对照。 |
| `tools/check_i06_d_reference.py`、`tools/verify_i06_d.py` | 跨语言校验、本地与 CI 统一验收入口。 |
| `.github/workflows/p1-a-i06-d.yml` | 独立分支的 Windows CI 和证据打包。 |
| `docs/p1-a/i06-d/` | 本报告及下列七份文档。 |

- [设计](I06-D-DESIGN.md)、[Schema](I06-D-SCHEMA.md)、[密码学](I06-D-CRYPTOGRAPHY.md)
- [测试报告](I06-D-TEST-REPORT.md)、[验收矩阵](I06-D-ACCEPTANCE-MATRIX.md)
- [安全审查](I06-D-SECURITY-REVIEW.md)、[回退与取证](I06-D-ROLLBACK.md)

冻结线上结构与六域合同保持原样。工程选择是精确范围、显式双时限、单目标虚拟读取、每次人工批准及模型内保留 nonce tombstone。生产密钥、可信时钟／身份、实际资源复核、外部副作用和恢复边界继续待决。D01–D05 保持待决，七款 Agent 均为 L0，五项标志均为 false。

独立复核先按 [验收矩阵](I06-D-ACCEPTANCE-MATRIX.md) 对照同一 SHA 的 Windows artifact、测试日志和冻结哈希。后续若获批准，应在隔离的 I06-S 试验中再验证角色来源、Prepared 变更传递、真实提交结果与进程绑定。
