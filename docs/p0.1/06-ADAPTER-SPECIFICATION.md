# Adapter 规范

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05

## 目的

把产品专属事件和 verdict 映射到统一协议，同时保持策略在 Core 中。本文为接口设计，没有创建 Hook、插件或可执行 Adapter。

## 结论

每种客户端/内核构建具有独立兼容 profile。Adapter 只做身份声明、传输、格式映射与证据采集，最终 risk、policy、grant 全由 Core 生成。没有已知阻止合同的产品只能登记或导入可验证的观察数据。

## 七款接入设计

| Agent | 候选集成 | 额外组件 | 首次安全状态 | 升级门槛 |
|---|---|---|---|---|
| Codex | 同步 PreToolUse + PermissionRequest + PostToolUse，app-server 自有实例为独立路线 | Hook shim，插件封装可选 | 未接入 | 内核版本、覆盖例外、timeout、write_stdin 后续输入、最终参数重写 |
| Claude Code | 同步 command Hook，支持版本显式 onFailure block | Hook shim，插件封装可选 | 未接入 | 精确构建、onFailure 生效、所有 matcher/子 Agent、无 bypass |
| DSH | Cordis tools guard/pre/result + subprocess/approval seam | 必需新开发桥接插件与外部 Adapter | 未接入 | bundle pin、guard 注册、PTC 子调用、plugin unload、Host 隔离 |
| 豆包 App 内工作模式 | 先确认 Windows 客户端身份和官方日志/扩展接口 | 未知 | 未接入 | 官方接口证据与可回滚接入，观察也需有真实来源 |
| Reasonix | 原生 Hook 最小路线，Extension v2 独立高级 profile | 原生 shim，深度集成另需 full-trust Extension | 未接入 | 原生/兼容 Hook 差异、required/slot、版本握手、权限不得提升 |
| WorkBuddy | 用户导出原生审计导入，桌面 Hook 仍调研 | 导入适配器，Hook 插件需求未定 | 未接入 | 桌面 payload/verdict、历史格式、版本、失效行为 |
| ZCode | 用户级同步 process Hook 或自有插件 | Hook shim，插件封装可选 | 未接入 | 新 session 快照、用户 enabled、项目忽略、timeout、子 Agent |

接入点来自独立调查与来源登记。上述“必需插件”指未来方案需要实现与验收，并非已有 AgentGuard 插件可安装。[能力调查](04-AGENT-CAPABILITY-MATRIX.md) [来源](13-SOURCE-REGISTER.md)

## Profile 必备内容

profile_id、agent_id、client_product、client_version、runtime_version、runtime_digest、publisher_signature、adapter_version、adapter_digest、protocol_major、tool_provider_identity、config_digest、runtime_generation、supported_events、blocking_events、approval_events、excluded_paths、timeout_behavior、error_behavior、native_fallback_behavior、last_test_report、expires_at。

声明一律由 Core 与已批准的兼容清单比对，Adapter 自报不能直接使状态从 L0 升到 L2/L3。只有本机受验证 metadata 能补充 installed_version，远程文档版本保留为独立字段。

对未知构建只允许 inventory/observational profile。原 profile 的 finite grant 不可跨 runtime 或 adapter 升级继承。

## 传输合同

| 消息 | 方向 | 主要字段/结果 |
|---|---|---|
| hello | Adapter → Core | 协议、版本、PID 声明、启动实例、连接挑战，返回 Core challenge |
| register_profile | Adapter → Core | 运行环境和能力摘要，返回经过核对的有效覆盖与降级原因 |
| report_event | Adapter → Core | source_event_id、sequence、event payload，返回 durable receipt 或明确拒绝 |
| evaluate | Adapter → Core | 原始 operation envelope、source invocation、截止时间，A返回pending/deny/defer_to_native，B只返回提交状态与prepared引用 |
| cancel | Adapter → Core | 真实 invocation、原因，不从自由文本取消任意任务 |
| heartbeat | 双向 | generation、last_source_seq、queue_loss、有效配置摘要 |
| verdict | Core → Adapter | A为decision_id与native deny/defer映射，B的permit仅由Broker在线兑换，Adapter不能兑换或直接启动 |
| execution_evidence | Adapter → Core | 结果与来源、process instance、observed changes，不自定证据等级 |
| shutdown | Core → Adapter | 停止新请求、revoke epoch、对账截止，不承诺外部进程已停止 |

Core 对 peer 角色实施方法白名单、帧长和速率限制。报告事件与获取 permit 的连接角色可分离，普通 sensor 无 evaluate 权限。

## 字段所有权

Ingress可提交原始command/arguments、agent声明、工具结果、source_event_id和血缘。Core按方法与角色读取允许字段，任何Adapter自定最终policy_decision/risk_level、认证true、grant状态或OS已验证身份均拒绝。

received_at、ingest_seq、真实peer实例、关联可信度、semantic/operation_digest、decision/grant与证书由Core/Broker生成。声明的Host PID与实际Adapter peer PID分开保存，仅连接Adapter不能证明其上游Host真实身份。

安全字段使用07的正式槽位，包括parent_invocation_id、subagent_id、depth与provider_id。extensions仅诊断字符串，不能存process/principal/network/grant/secret字段供策略使用。

## 生命周期

~~~mermaid
flowchart LR
  Discovered --> Handshaking
  Handshaking --> Observing
  Handshaking --> Gating
  Gating --> Degraded
  Observing --> Degraded
  Degraded --> Handshaking
  Observing --> Stopped
  Gating --> Stopped
~~~

心跳初始设计 5 秒，连续两次缺失进入 suspect，10 秒超过未恢复进入 degraded。请求路径不能等心跳超时才失败关闭，evaluate 无 Core 响应应在自身 deadline 立即拒绝。数值为待测提案。

## 同步时限和审批

Adapter宣告host真实budget，Core使用更短deadline。A没有明确安全决策时尽力返回该产品原生deny，只覆盖shim实际运行的路径，host启动失败/超时继续无法由shim补救。A无强绑定点时仅允许明确deny或交回原生确认，禁止后台连续自动批准。

B采用独立Broker的最终Prepare/Attest/Redeem/Launch合同，Agent工具必须真正委托并接收结果，不能同时执行原生工具。七款委托能力均未验证。Core或Broker不可达则不启动，不自动切回原生全权限工具。

审批预算不足时当前调用结束。默认不把后台“下一次相同操作”自动批准，延后意图仅在同一冻结prepared身份仍在、D04批准路线且AT17/19/30通过后另启用。新invocation与nonce必须重建，执行上下文变化需重新用户确认。

后置事件可异步，async Hook不能充当前置闸门。[Broker合同](15-EXECUTION-BROKER-CONTRACT.md) [精确编码](18-CRYPTOGRAPHIC-CONTRACT.md)

## 原生映射的差异

- Codex 和 Claude 使用各自 Hook JSON，不能复制不支持字段。Codex PreToolUse 的 ask 当前不受支持，PermissionRequest 是独立分支。
- DSH 守卫在 pre/approval 后运行，适合最终拒绝。需要检查后续 execute wrapper 是否还可改身份，宿主内恶意插件没有被外部 bridge 自动隔离。
- Reasonix 原生 Hook 用 exit code，PermissionRequest 本身无法回答审批。Extension permission.decision 是另一种 full-trust 合同，Guard 始终保留原生 deny，禁止返回可覆盖 host deny 的 allow。
- ZCode process argv 形式适合作为候选，用户级启用和新会话快照必须验证。项目级配置不能作为部署入口。
- WorkBuddy 与豆包没有获证的动态阻止合同，前置相关方法不注册，UI 不显示可“停止这个操作”的可用按钮。

接口事实及边界来自官方资料，具体证据版本见各调查表。[能力矩阵](04-AGENT-CAPABILITY-MATRIX.md)

## 结果、重连与乱序

Adapter 在完整接收 Core ack 前可重发同一 source_event_id 与完全相同内容。Core 以 source.instance_id + source.source_event_id 去重，内容不同即冲突并告警。不同报告通道无稳定 source ID 时只作 probable duplicate 标注，不删除原始事实。

断线不重放allow，依赖断线组件的未消费permit和证书失效。独立OS证书可在核对依赖后保留，不能一概清除。Core重启后core_boot_epoch更新，未决execution状态置unknown并对账。没有执行器的once-consume合同就不承诺“准确执行一次”。

子 Agent 使用 parent_session_id、parent_invocation_id、subagent_id、provider、depth 和有效 profile。外部 child 只记录委托事实，不把父级保护自动传递。

会话正常结束写 session.ended。非正常进程退出得到独立证据后才写agent.crashed，正常退出写session.ended，单纯心跳消失只能写 connection.lost。正在执行且无最终证据的操作保留 unknown。

## 部署与回滚设计

每个安装方案必须有用户可审查的逐项变更清单、安装前快照、命名空间 owner、文件摘要和回滚步骤。配置采用 compare-and-swap 保留同时发生的用户变更，仅移除本产品条目。插件卸载或 disable 后等待当前执行 generation 清空，重启新会话核实。回滚不恢复任何旧的长期许可。

P0 没有执行上述部署流程，P1 先在隔离副本验证再提出具体安装变更。

## 技术依据

各产品有不同事件名、字段、作用域和错误语义，统一层只负责明确映射。Named Pipe 与独立 Core 的边界来自总体架构。[架构](05-TECHNICAL-ARCHITECTURE.md) [S01、S07、S11–18、S24–39、S45](13-SOURCE-REGISTER.md)

## 风险

安装插件会引入新的信任代码，且 DSH/Reasonix 宿主内组件可处于沙箱外。Adapter 被污染可谎报事件，需独立证据与 OS 隔离。

## 尚未确定的内容

产品合同与安装版本映射、WorkBuddy/豆包接入、原生 allow 重试幂等性、最终参数重写顺序、进程归因权限和安全卸载的实际效果。


