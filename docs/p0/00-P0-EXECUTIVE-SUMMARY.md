# AgentGuard P0 执行摘要

项目 AgentGuard · V2.0 · Windows 11 · 2026-11-05（Asia/Shanghai）

## 目的

汇总七款接口调查、安全边界、总体架构、验收结论和P1依赖，提供设计交付入口。本文区分资料证据、设计判断与尚未执行的验证。

## 结论

**READY WITH CONDITIONS。**

P0文档设计可作为P1-A基础工程输入。真实Agent自动许可、执行前强制保护、系统级隔离与面向普通用户发布必须经过逐版本gate。当前已验证前置拦截为0款，已验证系统级隔离为0款。

若把目标解释为“无需Agent合作、普通用户态、对七款所有路径都能失败关闭并抗同权限恶意Host”，该强保证路线为 **BLOCKED**。本设计将它明确列为未满足目标，保留全部七款产品，按可验证能力分级推进。

## 七款真实调查结果

| Agent | 资料证明的接口 | 关键限制 | P1接入方向 |
|---|---|---|---|
| Codex | PreToolUse拒绝、PermissionRequest决策、PostToolUse，app-server审批 | Hook错误/超时可继续，write_stdin输入无新pre，hosted/专用路径缺口 | 版本专属shim，可靠执行边界另验 |
| Claude Code | 前置deny、权限决策、成功/失败post、MCP matcher、子Agent事件 | 默认故障多继续，v2.1.295+ command/http onFailure block需显式配置 | 优先可靠同步Hook profile |
| DSH | Cordis tools管线、monotonic guard、ctx.approval、PTC子调用 | 插件Host在沙箱外，可直接OS调用，master未映射安装包 | 新桥接插件、受控Host/执行边界 |
| 豆包App工作模式 | 目标按用户最新更正，具体Windows构建/事件接口未知 | 开放平台正文不可读，Skill不能作拦截证据 | 先确认客户端与官方接入，保持L0 |
| Reasonix | 原生PreToolUse exit2，Extension v2可拦截工具/权限 | 原生PermissionRequest不能答审批，spawn失败可继续，全信任Extension可越host限制，沙箱文档冲突 | 原生/Extension分profile并严格评审 |
| WorkBuddy | 桌面插件含Hook/MCP，原生安全中心和审计导出 | 桌面动态阻止/接管契约未证实，不能套CodeBuddy Code | 先审计导入与合同调查 |
| ZCode | 同步PreToolUse/PermissionRequest、post、插件Hook | 项目Hook被忽略，timeout最终行为/子Agent完整继承未知 | 用户级processHook，固定构建验证 |

五款有公开前置拒绝入口，分别是Codex、Claude Code、DSH、Reasonix和ZCode。入口存在不等于已验收。WorkBuddy可先规划观察/导入路线，豆包尚不能确认可用观察来源，不能笼统宣称两款已“仅观察接入”。[独立20项调查与矩阵](04-AGENT-CAPABILITY-MATRIX.md) [58条来源登记](13-SOURCE-REGISTER.md)

Codex和Claude可直接配置自有Hook shim，插件封装可选。DSH深度方案需要新增Cordis桥接插件，Reasonix深度方案需要经审核的runtime Extension，ZCode可选用户级Hook或插件。WorkBuddy/豆包需要什么额外插件仍未知，当前没有可安装的AgentGuard插件产物。

未确认公开契约的能力包括豆包目标模式的前后事件/本地阻止/审批接管、WorkBuddy桌面动态同步verdict，以及所有产品完整OS副作用与最终执行点一致性。未找到公开证据不能写成产品绝对不支持。

## 版本与证据

本地只读元数据为Codex Appx 26.1029.7803.0、Reasonix Studio2.56.0、WorkBuddy5.9.3、DSH主程序0.2.1.0/卸载器0.2.1-rc.2。官方页面显示Claude v2.1.318、Reasonix Studio v2.57.0、WorkBuddy5.11.0灰度及DSH v0.2.2-rc.1预发布。各版本不能互相替代，签名链与内核映射未验证。

DSH源码基准为fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159，Reasonix仓库基准为f4f32cc7d3078f7031c7852fbf0b7de410958199。它们是调研时分支HEAD，非本机release证明。E3/E4实验均未运行。[版本与来源](13-SOURCE-REGISTER.md)

## 最终架构及取舍

Tauri2 + React + TypeScript承载三类UI，独立Rust用户态Core承担确定性策略、有限授权和审计，独立Adapter连接各产品。Named Pipes采用显式身份/DACL/限流。SQLite单writer使授权计数和决策记录处于同一事务。第一版无默认Windows Service、无自研驱动、无默认管理员初始化。

策略与ledger共置，避免再拆policy进程引入消费/撤销分布式竞态。UI崩溃不决定Core生命周期。Tauri选择来自Rust复用和较小UI桥接需求，Electron具有成熟生态与固定Chromium优势，但本产品没有必须引入Node主进程的需求。实际资源性能未测。

AppContainer/受限令牌/ACL形成资源限制候选，Job负责受控进程生命周期。ETW/Sysmon仅作可选补充，Sysmon需额外service/driver。Windows Sandbox桌面产品不支持Home，Codex MXC是独立的新原生路线，本机适用性未知。[架构](05-TECHNICAL-ARCHITECTURE.md) [Windows边界](03-TRUST-BOUNDARIES.md) [16项ADR](11-TECHNOLOGY-DECISIONS.md)

## 核心安全设计

授权绑定命令、参数、目标、cwd、执行身份、工具/脚本/解释器、环境与沙箱策略摘要。变化使旧授权失效。单次nonce在最终执行点原子兑换，重启/撤销epoch失效旧许可。只返回普通allow且无最终消费接口的host不能获得强防重放承诺。

请求/收到/批准/执行/结果由不可变事件分别表达，证据区分Agent说法、受认证报告、独立观察、推断和未知。中文解释无决策权。默认本地脱敏审计，哈希链配独立锚点才增加部分篡改可发现性。[事件协议与8例](07-UNIFIED-EVENT-SCHEMA.md) [策略引擎](08-SECURITY-POLICY-ENGINE.md)

## 主要风险

1. Hook未启动、超时、升级或被绕过时，部分宿主仍可能执行。
2. 恶意Host/插件/MCP可在模型工具管线外操作，普通用户态监控器无法透明控制所有路径。
3. 同SID恶意进程能攻击Core、IPC、配置与日志，进程分离和本地链无法单独解决。
4. 路径TOCTOU、脚本/依赖变化、持久shell输入、网络重定向与后台子进程使简单摘要或前缀白名单不足。
5. Reasonix full-trust Extension具有权限提升语义，必须禁止Guard把原生deny提升为allow。
6. 文档与安装版本存在漂移，豆包/WorkBuddy/ZCode等关键合同未完整确认。

威胁模型登记22条攻击路径，覆盖用户指定的14类威胁来源及主要保护资产。[威胁模型](02-THREAT-MODEL.md)

## 尚未确定的内容

18项开放问题指定了关闭证据与阻塞范围。最高优先级为精确版本、豆包Windows客户端身份、WorkBuddy桌面合同、Reasonix沙箱/协议冲突、ZCode超时、最终执行点nonce和Windows出站/GUI隔离。[开放问题](12-OPEN-QUESTIONS.md)

## P0验收与P1准入

| 范围 | 判定 |
|---|---|
| 15份文件、7款独立调查、来源分级、威胁/边界/ADR/协议/UI/测试/P1计划 | 文档设计交付 |
| 七款真实运行兼容、前置阻止、故障关闭、OS隔离 | 未验收，未实测 |
| P1-A合成协议/策略/ledger/UI基础工程 | 具备设计输入，按条件推进 |
| P1-B真实Agent安装/Hook配置/隔离危险实验 | 本次未执行，需具体隔离方案与目标构建 |
| P1-C自动许可和L2/L3公开承诺 | 对应gate通过后准入 |
| 无合作全路径强制保护 | 当前BLOCKED |

本次只完成设计范围，未安装依赖、创建Hook、改Agent/Windows配置、读取凭证、启动其他Agent或连接LikeHub/云服务器，没有真实模型/API业务请求或危险测试。

## P1详细任务与依赖入口

共24个可直接转Issue的任务，给出责任、依赖、交付物与AT验收。主链为身份/证据 → 协议与IPC → 策略/ledger → 有限许可 → Windows边界与逐款适配 → 端到端认证 → 可回滚安装/可用性/供应链复核 → 分级发布。[完整P1计划](14-P1-DEVELOPMENT-PLAN.md)

测试计划包含40项用例与七款专属变体，要求独立sentinel、资产状态、网络sink及允许对照证明实际阻止。全部处于未执行状态。[测试与验收](10-TEST-AND-ACCEPTANCE-PLAN.md)

## 完整文件清单

| 文件 | 内容 |
|---|---|
| [00-P0-EXECUTIVE-SUMMARY.md](00-P0-EXECUTIVE-SUMMARY.md) | 汇报、结论和入口 |
| [01-PRODUCT-REQUIREMENTS.md](01-PRODUCT-REQUIREMENTS.md) | 产品合同与十项验收追踪 |
| [02-THREAT-MODEL.md](02-THREAT-MODEL.md) | 资产、对手、22项威胁与攻击图 |
| [03-TRUST-BOUNDARIES.md](03-TRUST-BOUNDARIES.md) | 信任域与Windows家庭版能力比较 |
| [04-AGENT-CAPABILITY-MATRIX.md](04-AGENT-CAPABILITY-MATRIX.md) | 七款矩阵与各20项调查 |
| [05-TECHNICAL-ARCHITECTURE.md](05-TECHNICAL-ARCHITECTURE.md) | 进程、数据流、IPC、队列和审计 |
| [06-ADAPTER-SPECIFICATION.md](06-ADAPTER-SPECIFICATION.md) | Profile、握手、映射与回滚合同 |
| [07-UNIFIED-EVENT-SCHEMA.md](07-UNIFIED-EVENT-SCHEMA.md) | 35字段Schema、生命周期与8例 |
| [08-SECURITY-POLICY-ENGINE.md](08-SECURITY-POLICY-ENGINE.md) | 确定性策略、有限授权与状态机 |
| [09-UI-UX-SPECIFICATION.md](09-UI-UX-SPECIFICATION.md) | 主界面、浮窗、弹窗及交互 |
| [10-TEST-AND-ACCEPTANCE-PLAN.md](10-TEST-AND-ACCEPTANCE-PLAN.md) | 40项测试、产品变体与分层gate |
| [11-TECHNOLOGY-DECISIONS.md](11-TECHNOLOGY-DECISIONS.md) | 16项ADR |
| [12-OPEN-QUESTIONS.md](12-OPEN-QUESTIONS.md) | 18项待解问题与责任 |
| [13-SOURCE-REGISTER.md](13-SOURCE-REGISTER.md) | 58条资料、需求和本地证据 |
| [14-P1-DEVELOPMENT-PLAN.md](14-P1-DEVELOPMENT-PLAN.md) | 24项Issue与依赖图 |

## 技术依据

关键能力以官方文档、固定仓库资料和有限本地元数据为依据。推导和提案在各文档中标明。文件中的数值默认值、UI尺寸、性能目标、队列和授权上限均未实测。[来源登记](13-SOURCE-REGISTER.md)

### 本次静态校验

已检查15份文件的五类必要章节、Markdown相对链接、代码围栏、来源编号、七款各20项调查，以及9个JSON块的语法。8个业务示例通过对本Schema使用的类型、required、额外字段、枚举、引用、边界、唯一性、UUID和时间格式的标准库检查。

环境没有独立完整JSON Schema验证器，本次未安装依赖，也未执行Draft 2020-12元Schema的独立验证。Mermaid已作文本结构检查，未渲染为图像。上述均为文档检查，不计入40项运行/危险测试的通过结果。

## 风险

READY WITH CONDITIONS只表示可作为分阶段开发设计输入。当前文档无法证明任何现有Agent已经受AgentGuard保护，强保护发布必须等待可复现的目标构建证据。

