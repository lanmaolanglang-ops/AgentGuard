# 保护等级与独立能力合同

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · AG-02 · 规范性基线

## 目的

统一等级、能力标志、证据、健康状态及失效规则。本文件是所有修订文档的术语依据，等级描述不能扩大认证范围。

## 结论

等级描述已验证的能力类别，五个能力标志描述独立认证结果。OS 隔离与 Hook 前置阻止独立，L2 不要求 OS 隔离，也不要求参数绑定或单次授权。L3 不推导所有 Hook 或所有资源受保护。

七款 Agent 当前实际有效等级均为 L0，五个标志全部 false。本文中的已验证状态示例仅用于合成协议测试，不是七款的运行认证。

## 固定定义

| 等级 | 唯一定义 | 必要依据 | 不从等级推导的能力 |
|---|---|---|---|
| L0 | 未接入 | 当前scope没有有效观察、前置阻止或OS隔离认证 | 参数绑定/单次认证可独立列出，不用其推定其它能力 |
| L1 | 事件观察已验证 | 当前范围 telemetry_verified=true，E3 事件源实验 | 前置阻止、参数绑定、OS 隔离 |
| L2 | 指定入口的执行前阻止已验证 | 当前范围 pre_execution_block_verified=true，E3 独立 body/sentinel 证据 | OS 隔离、强参数绑定、单次许可和全面 fail-closed |
| L3 | 指定范围的 Windows OS 隔离已验证 | 当前范围 os_isolation_verified=true，E4 负面边界实验 | Hook 可用、语义完整性、参数绑定、任意域名白名单 |

等级为同一个 scope 内有效能力的展示摘要，按 OS、前置、观察三个认证类别选择 L3、L2、L1，否则 L0。scope 不同的证书不能拼成更高等级，同一卡片可并列“工具入口 L2”“受控 worker 文件边界 L3”，禁止只显示一个无范围的总绿灯。

## 五个独立标志

| 标志 | 认证对象与验收 | 证据门槛 | 实现任务 |
|---|---|---|---|
| telemetry_verified | 给定来源/事件集的归因、时序、丢失识别与可观察性 | E3，AT21/24/40 | I03、I08、I21、目标 Adapter |
| pre_execution_block_verified | 对指定入口显式拒绝后工具 body 未进入 | E3，AT02/14/15/28 的相应子用例 | I10–I16、I17 |
| parameter_binding_verified | 最终执行参数/身份/对象与已批准快照一致，变化拒绝 | E3，AT19/30/44 | I06、I07-B、目标委托路线 |
| single_use_grant_verified | nonce 原子消费、并发/重试/崩溃无第二次启动授权 | E3，AT17/18/22/36/45 | I05、I06、I22 |
| os_isolation_verified | 指定资源/进程树/GUI/网络边界的越界尝试实际被 OS 限制 | E4，AT08/09/25/28/38/42/43/44 中声明范围的全部项目 | I07、I17 |

parameter_binding_verified 可独立认证而 single_use_grant_verified=false，反之亦然。只有两个标志与委托路线、原生许可、所需资源强制点全部满足，才可考虑连续有限自动授权。等级不参与这个许可条件，禁止用 level>=L2 代替明确条件。

## 认证记录与作用域

每条 CapabilityCertification 必须包含 certificate_id、claim_scope_id、agent_id、client/runtime/adapter/provider/config 摘要、entrypoint_ids、execution_mode（A/B/C）、资源范围、five flags、evidence_refs、evidence_grade、OS build/补丁、test_fixture_digest、issued_at、expires_at、verified_by、fault_behavior 和 effective_state。

五个标志存储“当前有效”的布尔值。证书原始结果为不可变历史，effective_state 可为 valid、expired、invalidated、unavailable、unknown。任何证据缺失或状态不为 valid，当前标志为 false，历史结果保留在证书记录中。有效证书的示例不赋予真实 Agent 新证据。

fault_behavior 为 verified_fail_closed、known_fail_open 或 unknown，按精确 fault 和入口登记。显式 deny 通过可获得健康入口的有限 L2，已知 host fail-open 不因此被掩盖。UI 显示“前置拒绝已验证，Hook 故障时可能继续”，不得宣称持续 fail-closed。未能检测的 Hook 短暂失效是已公开盲区，心跳不能消除它。

## 更新、断线和局部降级

| 变化 | 失效对象 | 允许保留的独立能力 |
|---|---|---|
| Agent/runtime/provider 更新 | 与该摘要绑定的 telemetry/pre/parameter/single-use 证书失效 | 与外部 OS policy 独立且主体/worker digest 均未变化的隔离证书，经再次核对后可保留 |
| Adapter 更新或丢事件 | 对应事件/入口证书失效，原 grant 失效 | 无依赖该 Adapter 的 OS 证书可保留 |
| 安全配置或沙箱策略变化 | 所有依赖该摘要的证书与许可失效 | 其它 scope 中无依赖证书可保留 |
| Hook 失效但 OS 边界仍有独立证据 | pre/相关telemetry=false | os=true，显示“指定范围 L3，Hook 不可用”，不能降成“无隔离”或显示完整保护 |
| OS 强制点健康未知 | os=false，阻止依赖该边界的新 B 许可 | 已独立验证的 A 前置拒绝可保留 |
| Core 重启 | 新core_boot_epoch，旧未消费许可全部invalidated | 证书须核对当前依赖与有效期，不自动继承旧effective flags |
| 证书过期 | 对应 flag=false | 其它有效独立证书仍可用 |

撤销能力证书与撤销 grant 分开。证书失效影响新许可，不能把已经发生的执行或网络上传抹去。

## A/B/C 认证规则

A 为原生 Hook 前置阻止，B 为操作委托给 AgentGuard Execution Broker，C 为 OS 独立资源限制。A、B、C 可同时存在于同一产品的不同范围。每种路线单独填写已覆盖/排除入口和验收报告。

A 没有最终点时可保留有限 L2，只允许确定性 deny 或交回真实原生审批，禁连续自动允许。B 无有效最终参数和nonce认证时禁止自动执行，不能回退A后继续同一permit。C 的认证必须列资源维度，例如“文件边界已验，网络未验”，网络无证据时不声称网络隔离。

## UI 必须展示的合成例子

“L2 · Bash 前置拒绝已验证 · 参数绑定未验证 · 单次许可未验证 · 故障可能继续”。

“L3 · Worker-1 文件范围隔离已验证 · 网络未验证 · Hook 不可用”。

两个例子均属于设计状态说明。真实七款的初始状态统一为“L0 · 未接入 · 所有标志 false”。

## 技术依据

规范定义来自AG-02，验证分离依据原生接口差异与Windows资源边界。协议字段见事件Schema，认证用例见测试计划。[协议](07-UNIFIED-EVENT-SCHEMA.md) [测试](10-TEST-AND-ACCEPTANCE-PLAN.md) [能力证据](04-AGENT-CAPABILITY-MATRIX.md)

## 风险

等级摘要容易掩盖范围，五个true也不能证明未列出的入口。已知fail-open必须持续可见，无法侦测的配置旁路保留为威胁。

## 尚未确定的内容

有效期、重新认证周期、各Agent独立范围及E3/E4报告均待P1。本文未批准任何真实产品升级保护等级。

