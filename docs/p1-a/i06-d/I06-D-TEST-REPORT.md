# I06-D 测试报告

在 Windows 工作树运行 `python -B tools/verify_i06_d.py`。同入口用于 GitHub Actions。证据目录为 `.work/i06-d`，运行结果和具体测试名记录在 `summary.json`，Rust 日志为 `typed-tests.log`。

| 核验 | 对照与证据 |
|---|---|
| 结构 | 有效 Permit 在键反序后消费成功。缺字段、多字段、重复键、错误类型、非法日期、越界整数、错误 null、浮点词法、超长及非法 MAC 均拒绝，仍保留原有效状态。 |
| 密码学 | 21 组冻结向量的编码、帧和 MAC 由三语言对齐。额外 6 组完整域载荷核对，覆盖实签 Permit 与 audit、intent、secret。 |
| 绑定 | 逐一修改快照、主体、配置、准备代次和上下文。重新签名的替换 Permit 也因原预占绑定不符被拒绝。 |
| 状态与故障 | 64 组状态对，15 合法、49 拒绝。涵盖额度竞争、nonce 碰撞、重复兑换／撤销、确认丢失、拒绝提交、失效后恢复旧输入。 |
| 时间与轮换 | UTC 与单调截止点独立测试。覆盖 TTL 两端、时钟回拨、时钟不可用、独立 epoch、boot 切换、旧 boot 重新出现及密钥材料重用。 |
| 回归 | 同入口执行 G0、I05、C2 和第一批回归。新 D 测试函数、既有测试函数、状态对和密码学向量分别统计。 |

`summary.json` 记录运行 commit、环境、锁文件 SHA-256、42 文件 manifest 摘要、各项 PASS 和原有回归证据。Windows 下载 ZIP 后用 GitHub artifact API 的 SHA-256 与本地计算值交叉校验。上传只包含测试日志、冻结清单和结构化证据。可按分支在运行列表查找同一提交。[I06-D Windows CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/workflows/p1-a-i06-d.yml?query=branch%3Ap1-a%2Fi06-d-typed-contract)
