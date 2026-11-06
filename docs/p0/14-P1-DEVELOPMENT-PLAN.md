# P1 开发计划、Issue 拆分与依赖

AgentGuard V2.0 · P0 交付 · 2026-11-05

## 目的

把设计拆成可以直接录入 Issue 的任务，明确输入、依赖、交付物、验收条件与阶段门槛。以下全部为计划，当前未创建远端 Issue、未实现业务功能。

## 结论

分 P1-A 基础合同与合成验证、P1-B 隔离环境真实兼容验证、P1-C 受限试用发布。首先打通一条具备可靠失败关闭的受管路径，其余产品继续登记和逐款推进。七款均保留，没有将未知产品移出范围。

P1-A 可以在本次设计基础上立项。P1-B 的真实Agent部署与实验必须先形成具体隔离计划，P1-C 必须完成对应L1/L2/L3门槛。不存在“UI上线即七款全面保护”的交付项。

## 可直接建立的 Issue

| ID / 标题 / 责任 | 依赖 | 实施范围与交付物 | 验收条件 |
|---|---|---|---|
| I01 固定产品身份与证据包 · 兼容/QA | 无 | 七款client/runtime/provider/version/signature清单，脱敏fixture与文档映射，维护来源更新 | Q01/Q13有逐款状态，未知不补造，WorkBuddy/CodeBuddy及豆包模式明确分离 |
| I02 构建合成安全夹具 · QA/Windows | I01部分结果 | 假文件树、body sentinel、受控网络sink、允许/拒绝对照、故障注入设计 | S/V环境与真实数据隔离，正向对照可观察，AT01–13可重置 |
| I03 统一协议与IPC合同 · Rust | I01 | schema1.0、ingress权限、Named Pipe身份、帧限额、去重/乱序模块 | 8示例及错误fixture通过，AT21/23/29/34，Adapter伪造allow被拒 |
| I04 确定性分类与硬禁 · 安全/Rust | I03 | typed operation、风险分类、硬禁、未知fallback、可审查recipe | AT01–07/19/26，unknown不能自动allow，无仅命令名白名单 |
| I05 SQLite审计与授权账本 · Rust | I03 | 单writer、事务边界、durability、busy/满盘/checkpoint、schema迁移与恢复 | AT21/22/29/36，grant消费与审计原子提交，异常无许可 |
| I06 有限授权、nonce与撤销 · 安全/Rust | I04/I05 | operation_digest、pending快照、一次性兑换、epoch、scope和deadline | AT17–19/22/30/36/39，字段逐一变化失效，100并发仅一次消费 |
| I07 Windows执行边界PoC · Windows安全 | I02/I06 | 受限token/AppContainer/ACL/Job、最小句柄、受控launcher、文件/网络负面测试 | AT08–10/25/28/30/31/38，设置失败无无隔离回退，证明可用平台 |
| I08 审计链、脱敏与回放投影 · Rust/安全 | I03/I05 | 脱敏append-only、段封口与锚点导出、来源分级、未知结果投影 | AT21/24/27/34/40，回放无执行副作用，不能把自报成功当独立核验 |
| I09 Tauri三界面与审批状态 · UI/UX | I03/I06/I08 | 主界面、浮窗、高危弹窗、有限授权表单、能力与降级呈现 | UI无任意shell/DB写，A01–A07固定显示，AT16/19/32/37 |
| I10 Codex版本专属适配 · 兼容 | I01–03/I06，L3另需I07 | Hook shim与app-server分profile、统一执行工具/MCP/patch/子Agent、排除持久输入和hosted路径 | AT14/15/17/19/20/33/39及专属变体，fail-open不能单独获L2 |
| I11 Claude Code适配 · 兼容 | I01–03/I06 | 同步Hook、onFailure=block版本验证、MCP/子Agent/作用域映射与安全卸载 | v2.1.295+目标构建pin，AT12–20/35/39，所有异常类型逐项报告 |
| I12 DSH Cordis桥接适配 · DSH/安全 | I01–03/I06，Host隔离依赖I07 | 版本化guard/pre/result、PTC嵌套、approval/subprocess、preset继承与plugin失载 | AT12–20/25/28/33，固定release bundle映射，Host直接OS旁路有明确结论 |
| I13 豆包工作模式接口收敛 · 兼容/产品 | I01 | 确认Windows客户端构建，官方接口/日志来源与可回滚方案，有证据才实现观察或闸门 | Q02/Q13证据充分，缺接口保持L0且不删除A04，不套独立豆包工作Skill |
| I14 Reasonix双路线适配 · 兼容/安全 | I01–03/I06/I23 | 原生Hook与Extension v2分profile，full-trust评审、required/slot行为与Windows沙箱验证 | AT13–20/25/28，host deny绝不被Guard allow提升，关闭Q05/Q06或禁相关能力 |
| I15 WorkBuddy桌面适配 · 兼容 | I01/I03/I23 | 审计导入PoC、桌面Hook合同核对、全局full-access与本地/云任务区分 | AT20/21/24/35，CodeBuddy结果不计入，未证实前置接口不得展示L2 |
| I16 ZCode用户级适配 · 兼容 | I01–03/I06 | processHook/插件映射、配置快照、project ignored、timeout及子Agent/MCP | AT12–20/35/39，固定构建，未知timeout阻塞L2 |
| I17 端到端安全回归与版本认证 · QA/安全 | I02–08，逐款依赖I10–16 | 按profile执行40项相关用例、独立副作用证据、升级/卸载认证报告 | 每项pass/fail/skip+原因，无危险跳过却宣称整体通过，L2/L3门槛分开 |
| I18 安装、配置CAS与回滚 · 发布/Rust | I01/I03，逐款合同确认 | 用户级安装包、签名验证、具体配置diff、自有命名空间、卸载恢复与并发保护 | AT35，保留用户Hook，无PATH/代理/证书/服务默认变更，失败可恢复 |
| I19 非技术用户与无障碍验收 · UX/QA | I09 | 构造风险任务、DPI/多屏/键盘/读屏、通知节奏与授权默认值 | AT32/37及≥5人理解测试，严重误判修正，默认值经审查 |
| I20 供应链、性能和独立安全复核 · 安全/发布 | I03–09/I18 | lockfile/SBOM/签名/最低版本、FFI审查、SQLite版本缺陷核对、七产品压测 | AT23/27/29/34，目标延迟实测，未测指标不写承诺 |
| I21 可用sensor与覆盖报告 · Windows/QA | I01/I03/I08 | 普通用户可见进程/变更/网络证据，权限探测、gap和数据最小化 | 不需新driver/service，读文件全量不可得明确显示，AT21/24/31/40 |
| I22 Core独立生命周期与恢复 · Rust/Windows | I03/I05/I06 | UI退出/Core后台、Job句柄所有权、restart epoch、未决对账、退出防护流程 | AT16/18/33/36/40，Core重启无旧许可，UI关闭不意外停止策略 |
| I23 关闭官方文档/版本冲突 · 兼容/安全 | I01 | Reasonix native/compat/v2和Windows冲突、ZCode配置冲突、WorkBuddy桌面合同证据报告 | 逐项有固定release证据或标不具备该保护能力，不选更乐观说明 |
| I24 分级试用发布审查 · 安全/产品 | I17–22及目标适配完成 | 七款状态清单、保护范围、例外、恢复/卸载指南、未决风险与发布决定 | G-L1/L2/L3逐项满足，任何未闭P0关键风险阻止相应自动许可 |

I01–I09/I20/I22 的合成实现与合同验证为 P1-A。I02 的隔离环境实际配置、I07 的 OS 实验、I10–I16 的真实部署和 I17 的危险测试属于 P1-B，需预先形成具体隔离方案。表中依赖不是授权执行这些操作的确认。

## 依赖图

~~~mermaid
flowchart TD
  I01["I01 身份与证据"] --> I02["I02 夹具"]
  I01 --> I03["I03 协议与IPC"]
  I03 --> I04["I04 策略"]
  I03 --> I05["I05 账本"]
  I04 --> I06["I06 有限许可"]
  I05 --> I06
  I02 --> I07["I07 Windows边界"]
  I06 --> I07
  I05 --> I08["I08 审计"]
  I06 --> I09["I09 UI"]
  I08 --> I09
  I01 --> I23["I23 冲突收敛"]
  I23 --> AD["I10–I16 逐款适配"]
  I06 --> AD
  I03 --> I21["I21 观察与覆盖"]
  I08 --> I21
  I06 --> I22["I22 生命周期"]
  I09 --> I19["I19 可用性"]
  AD --> I18["I18 可回滚安装"]
  AD --> I17["I17 端到端认证"]
  I07 --> I17
  I21 --> I17
  I22 --> I17
  I18 --> I20["I20 供应链与性能"]
  I17 --> I24["I24 发布门槛"]
  I19 --> I24
  I20 --> I24
~~~

各适配器可在自己的合同证据完备后推进。跨产品并行开发不意味着安全规则或授权账本可有多个独立决策者。图为里程碑依赖概览，逐项精确依赖以 Issue 表为准。

## 完成定义

每个Issue包含可审查diff、来源/版本、适用威胁和AT编号、正反例、故障路径、隐私检查、恢复方法与未覆盖项。实际接入报告必须区分“接口存在”“fixture通过”“目标构建通过”“OS边界通过”。

发布清单必须固定七行，允许各行有不同状态。无法接入的产品交付明确状态与证据缺口，不能伪造观察数据或自动删除该产品。

## 技术依据

Issue范围来自需求、能力差异、威胁与授权不变量。具体门槛已在测试计划定义。[需求](01-PRODUCT-REQUIREMENTS.md) [开放问题](12-OPEN-QUESTIONS.md) [测试计划](10-TEST-AND-ACCEPTANCE-PLAN.md)

## 风险

外部产品合同和版本变动影响开发量。当前没有团队人数、工期与预算输入，不提供未经估算的承诺日期。完整强隔离对七款的可行性仍可能需要更改高级路线。

## 尚未确定的内容

各Issue工作量、目标release和实验环境。若最终执行点无法参与nonce/参数复核，相关自动许可目标保持BLOCKED，产品仍可按已验观察/受管拒绝范围推进。

