# P1-A第一批新问题

AgentGuard V2.0 · 2026-11-06

| 编号 | 发现 | 处理与证据 | 设计影响 |
|---|---|---|---|
| NI01 | 原验证环境FormatChecker缺date-time可选依赖，可能静默跳过 | 新增rfc3339-validator/six锁定，Python与Rust均加入畸形时间拒绝回归，13 Python/14 Rust测试通过 | 修复验证实现，冻结规范不变 |
| NI02 | 缓存Tokio 1.53.1命中RUSTSEC-2026-0386 | 选修补1.54.1并锁定，OSV复查111依赖版本无剩余公告 | 开发依赖调整，P0.1未固定crate版本 |
| NI03 | ZCode选定PE只有Electron 41.0.3且NotSigned | 记录原始元数据与hash，不填写产品runtime版本，不升级能力 | 继续L0，I16前置缺口 |
| NI04 | Reasonix固定GUIDE同时存在Windows opt-in隔离说明和旧无沙箱段落，CONFIG也有旧描述 | 固定commit/API片段与hash保留，相关能力关闭 | I23可交付明确关闭，Q05/06继续待真实映射 |

D01–D05原重大决策全部保留，本批没有需要修改冻结字段或降低安全门槛的变更提案。
