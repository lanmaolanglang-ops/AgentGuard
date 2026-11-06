# AgentGuard P0.1执行摘要

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · Windows 11

## 结论

**CONDITIONAL PASS**。本次形成安全设计修订基线，真实七款前置拦截认证和OS隔离认证仍为0。P1-A具备分阶段开发输入，P1-B等待隔离环境及实验，P1-C当前BLOCKED。

原P0的15份文件只读保留。P0.1由15份同名修订、6份技术补充和6份治理/验收文档组成，共27份Markdown。没有程序实现、安装、Agent配置或系统策略变更。

## 修订后的控制链

三种路线独立认证。A为Agent原生Hook前置阻止，B为委托独立Broker/Worker，C为OS资源限制。七款B端到端委托均待验证。A无最终执行点可取得有限显式deny认证，但无强参数绑定/单次消费、禁连续自动批准，也不能覆盖原生deny。

Broker由Prepare固定最终操作和OS身份，Core独占密钥及ledger，Attest后在线Redeem。消费与审计同一事务，Broker只启动同一prepared操作。消费后执行失败保留grant.consumed，执行结果另记。崩溃可能导致unknown或损失一次额度，不承诺OS副作用exactly-once。

[执行合同](15-EXECUTION-BROKER-CONTRACT.md) [总体架构](05-TECHNICAL-ARCHITECTURE.md)

## 保护等级

L0未接入，L1事件观察已验证，L2指定入口的执行前阻止已验证，L3指定范围的Windows OS隔离已验证。telemetry、pre_execution_block、parameter_binding、single_use_grant、os_isolation五标志各自认证。

L2不要求OS或强参数绑定。L3不推定Hook可用。当前七款全部L0、五标志false。每个证书绑定精确scope、构建、配置、入口、资源、证据与期限，故障及更新仅使对应依赖失效。[唯一分级合同](16-PROTECTION-CAPABILITY-CONTRACT.md)

## 七款证据冻结

| ID | Agent | 原有最高证据 | 主要缺口 |
|---|---|---|---|
| A01 | Codex | E1 | fail-open、hosted/opt-out、持久shell输入与内核映射 |
| A02 | Claude Code | E1 | 文档版本不等于本机，onFailure/作用域待实际验证 |
| A03 | DeepSeek Harness | E2 | release映射、插件直接OS、guard不是OS边界 |
| A04 | 豆包 App 内工作模式 | E0 | 用户确认模式，Windows客户端构建/接口未知 |
| A05 | Reasonix | E2 | 原生Hook与Extension v2分开，Windows沙箱资料冲突 |
| A06 | WorkBuddy | E1 | 桌面verdict合同未知，不能使用CodeBuddy结果 |
| A07 | ZCode | E1 | 项目Hook忽略、具体构建/timeout/子Agent覆盖未知 |

E0为未证实，E1官方正文，E2固定仓库文档/源码静态，E3精确运行实验，E4 OS边界负测。本轮协议和密码学合成验证不提升Agent证据。[各20项调查](04-AGENT-CAPABILITY-MATRIX.md) [来源](13-SOURCE-REGISTER.md)

## Windows与威胁边界

T2-A仅指确实纳入受限环境的Host。T2-B为同SID/相近IL恶意进程，普通用户Core、Job和DPAPI不能全面抵御。独立主体/服务取舍保留D01。

网络观察、请求代理和OS强制分开。AppContainer/LPAC/Firewall/WFP均待实验，代理变量、域名匹配与Internet capability不构成全面域名白名单隔离。14项网络向量和GUI/COM/桌面/句柄兼容矩阵均列独立实验。

[威胁模型](02-THREAT-MODEL.md) [Windows边界](03-TRUST-BOUNDARIES.md) [网络](17-WINDOWS-NETWORK-ISOLATION.md) [GUI PoC](19-WINDOWS-ISOLATION-POC.md)

## 协议与验证

协议升为2.0.0，Draft2020-12闭合字段，正式身份、网络、grant、capability及permit，extensions只诊断。旧1.0只导入观察，迁移不继承许可。

9份Schema和65个夹具已按标准验证器检查，含37正例与28预期拒绝反例，另有8迁移、64状态对、严格parser和兼容性检查。21组typed encoding/frame/HMAC向量由Python/Node独立一致。Rust验证与CI接线仍为P1工作，当前无Git仓库且本次仅Markdown，不能声称CI部署完成。

[正式数据合同](07-UNIFIED-EVENT-SCHEMA.md) [密码学](18-CRYPTOGRAPHIC-CONTRACT.md) [可执行验证入口](20-PROTOCOL-VALIDATION-FIXTURES.md)

## 开发和发布

24个父任务保留，I02/I07/I20各拆两个阶段，总27节点。精确117条依赖由同一数据源生成图表。I07-A早期隔离实验只依赖隔离环境和早期供应链，I17汇合Adapter、UI、安装、sensor、生命周期及边界真实结果，I24负责发布准入。

AT01–40保留，增加AT41同SID、AT42 GUI、AT43网络、AT44句柄/IPC、AT45授权审计、AT46更新降级。真实危险测试全部未执行。

[任务计划](14-P1-DEVELOPMENT-PLAN.md) [测试计划](10-TEST-AND-ACCEPTANCE-PLAN.md)

## 需要负责人决定

D01独立安全主体，D02网络backend/权限，D03 GUI隔离/兼容，D04首个委托Agent/recipe与shell范围，D05密钥存储/反回退信任锚。共5项未决，本次只给推荐与可逆实验，没有替负责人选定部署方案。

[决策日志](P0.1-DECISION-LOG.md)

## 交付入口

所有文件、用途、关系和最终校验结果见索引与验收报告。AG-01–AG-10均有规范修复、对应Issue/测试及剩余实验条件。另识别12项跨文档问题并逐项修复。

[文档索引](P0.1-DOCUMENT-INDEX.md) [变更](P0.1-CHANGELOG.md) [追踪](P0.1-TRACEABILITY.md) [交叉审查](P0.1-CROSS-DOCUMENT-AUDIT.md) [验收](P0.1-ACCEPTANCE-REPORT.md)

