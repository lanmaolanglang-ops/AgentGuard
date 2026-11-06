# 七款 Agent 能力矩阵与独立调查

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · 保留P0原始来源与调查边界

## 目的

确定七款产品的真实身份、接口契约和适配边界，为后续安全设计提供依据。所有 S 编号在来源登记中给出名称、URL、版本、说明及可信度。本文中的“已证实”表示官方资料或固定源码证明该接口存在，部署有效性另行验收。[来源登记](13-SOURCE-REGISTER.md)

## 结论

Codex、Claude Code、DSH、Reasonix、ZCode 存在有据可查的前置拒绝入口。五者的覆盖面与错误语义不同。WorkBuddy 桌面有 Hook 插件类型和原生权限控制，第三方动态阻止协议未证实。A04 按用户最新纠正锁定为“豆包 App 内的工作模式”，Windows 客户端内该模式是本项目的暂定解释，具体客户端构建未知。

本次没有启动 Agent、安装 Hook 或执行接口测试。七款在 AgentGuard 中的实际接入状态均为“未接入”，已验证执行前拦截和已验证系统级隔离的数量均为零。

## 证据与状态口径

| 等级 | 含义 | 可否据此显示已验证保护 |
|---|---|---|
| E0 | 未取得正文、身份/版本未知、检索线索 | 否 |
| E1 | 已打开的官方文档/Release 明确描述 | 否 |
| E2 | 固定提交的官方仓库文档或源码静态证据，逐项注明 | 否 |
| E3 | 精确构建与配置下的可复现实验、独立副作用证据 | 可限实验覆盖面显示 |
| E4 | OS 隔离负面测试及绕过测试通过，版本与执行链锁定 | 可限隔离边界显示 |

状态仅使用已证实、部分支持、未证实、不支持、不适用。“未证实”不能转写成“不支持”。观察、拒绝建议、实际阻止、审批接管分别验收。

## 完整矩阵

| Agent | 版本 | 事件观察 | 执行前阻止 | 审批接管 | 执行后记录 | 子 Agent 覆盖 | MCP 覆盖 | 失败时行为 | 证据等级 |
|---|---|---|---|---|---|---|---|---|---|
| A01 OpenAI Codex | 桌面包 26.1029.7803.0，内核版本未知 | 部分支持 | 部分支持 | 部分支持 | 部分支持 | 部分支持 | 部分支持 | 已证实，部分 Hook 错误/超时可继续工具 | E1，S01–06、S57–58 |
| A02 Claude Code | 官方发布 v2.1.318，本机版本未知 | 已证实 | 已证实，限匹配工具入口 | 已证实，限 PermissionRequest | 已证实，成功/失败事件分开 | 部分支持 | 部分支持 | 已证实，默认通常继续，v2.1.295+ 可配置 onFailure block | E1，S07–10 |
| A03 DeepSeek Harness | 本机主程序 ProductVersion 0.2.1.0，卸载器 0.2.1-rc.2，公开候选 v0.2.2-rc.1 | 已证实 | 已证实，限受管 pipeline | 部分支持，ctx.approval 与 guard seam | 已证实 | 部分支持 | 部分支持 | 部分支持，pre/guard 抛错规范化失败，失载/旁路未知 | E2，S11–20，未映射安装版本 |
| A04 豆包 App 内工作模式 | 身份按用户指定，Windows 构建未知 | 未证实 | 未证实 | 未证实 | 未证实 | 未证实 | 未证实 | 未证实 | E0，S21–22 仅消歧与检索边界 |
| A05 Reasonix | 本机 Studio 2.56.0，公开 Studio 2.57.0 | 已证实 | 已证实，原生 PreToolUse exit 2 | 部分支持，仅 full-trust Extension 路线有契约 | 已证实 | 部分支持 | 部分支持 | 已证实，原生 timeout 阻止但 spawn/崩溃可继续，Extension 另有规则 | E2，S23–30，未映射安装版本 |
| A06 WorkBuddy | 本机 5.9.3，官方另列 5.11.0 灰度 | 部分支持，原生审计与任务 UI | 未证实，第三方接口 | 未证实 | 部分支持，原生审计可导出 | 未证实，第三方事件覆盖 | 部分支持，产品有 MCP | 未证实，桌面 Hook 契约 | E1，S31–34 |
| A07 ZCode | Windows 安装版本及文档对应构建未知 | 已证实 | 已证实，限同步工具 Hook | 已证实，限需确认的请求 | 已证实 | 未证实，完整 Hook 继承 | 部分支持 | 部分支持，可恢复错误已描述，超时最终 verdict 未证实 | E1，S35–39 |

A01 的 hosted tools、专用 opt-out 路径及 write_stdin 后续输入不在统一前置覆盖承诺内。A03/A05 的 E2 主要是固定仓库文档，DSH guard 另读了实际源码片段。表内任何 E1/E2 均不能用于宣称本机已经拦截。

## A01 · OpenAI Codex

| 调查项 | 结果、接口与局限 |
|---|---|
| 1 官方身份 | OpenAI Codex，桌面、CLI、app-server 分开建 profile。OpenAI.Codex 包身份已查到，未从包号推算 CLI 内核版本 |
| 2 当前版本/获取 | 本地 Appx 元数据版本 26.1029.7803.0。后续读取 About 与内核签名/构建元数据，不在 P0 启动 CLI |
| 3 安装 | 官方桌面分发及 CLI 分发分别登记。本机是 Appx 包，安装流程未执行 |
| 4 插件 | 可携带技能、MCP、生命周期 Hook，安装和 Hook 信任分开。新增插件是 P1 可选交付形式 |
| 5 Hook | PreToolUse、PermissionRequest、PostToolUse，以及 session/subagent 事件 |
| 6 生命周期 | tool proposal → pre → 可能的原生审批 → tool output → post。前置通知不代表进程已经启动 |
| 7 前置阻止 | 支持 permissionDecision deny 或 exit 2。unsupported ask/continue 等字段可能报错后继续，不照搬 Claude JSON |
| 8 执行后 | PostToolUse 覆盖支持工具，Unified exec 的完成可能在后续 write_stdin 轮询才上报 |
| 9 权限申请 | PermissionRequest 仅发生于本来需要审批的操作，无法替代全量 pre |
| 10 接管审批 | Hook 可 allow/deny。app-server client 也能响应其所连接实例的审批，未证实可附加接管正在运行的桌面会话 |
| 11 限制工具 | 使用受支持的 deny 和原生权限/规则组合，不能依赖模型遵守说明 |
| 12 子 Agent | 有生命周期事件，具体会话 ID、继承与工具路径需分构建验证 |
| 13 MCP | 常规 MCP 工具进入本地工具 Hook，远端服务实际副作用不由本地 Hook 单独证明 |
| 14 绕过路径 | hosted tools、专用 opt-out，持久 shell 的 write_stdin 后续输入，工具内部 subprocess/网络及已有长任务 |
| 15 超时 | 大多数当前 Hook 默认 600 秒，SessionEnd/Interrupt 预算更短。安全候选请求必须显式配置预算并测实际行为 |
| 16 异常 | 官方说明回调错误、超时、畸形响应可能令 Hook 失败而继续工具。仅 Hook 的 fail-closed 保证不成立 |
| 17 配置 | 用户/项目 .codex/hooks.json 或 config.toml。项目需信任，插件 hooks/hooks.json 也可被加载 |
| 18 日志历史 | Hook 输入可有 transcript_path，格式不稳定。history.persistence/history.jsonl 与 rollout 应分开解释，实际桌面存储位置未核实 |
| 19 更新 | 滚动文档与 release/schema 可能不同。按内核、客户端、工具提供方、配置摘要分别 pin |
| 20 卸载回滚 | 设计上移除本产品命名空间 Hook 或禁用其插件，保留其他 Hook，恢复经批准的安装前副本，重启会话复核。未验证桌面卸载流程 |

主要依据为当前 OpenAI Hooks、配置、插件和 App Server 文档。结论适用于文档描述的路径，安装构建仍需验证。[S01](https://learn.chatgpt.com/docs/hooks) [S04](https://learn.chatgpt.com/docs/app-server) [S05](https://developers.openai.com/plugins/build/plugins) [S06](https://learn.chatgpt.com/docs/config-file/config-advanced)

沙箱与审批独立。调查覆盖旧 sandbox_mode 的 read-only/workspace-write/danger-full-access、新权限 profile 与 on-request/never/granular。新旧配置不能任意混用。Windows 当前文档新增 MXC 路线，不能继续使用“Windows 只能 elevated/unelevated”的旧假设。[S02](https://learn.chatgpt.com/docs/agent-approvals-security) [S03](https://learn.chatgpt.com/docs/windows/windows-sandbox) [S57](https://learn.chatgpt.com/docs/permissions)

## A02 · Claude Code

| 调查项 | 结果、接口与局限 |
|---|---|
| 1 官方身份 | Anthropic Claude Code，CLI/桌面执行位置独立确认 |
| 2 当前版本/获取 | 官方 latest 重定向 v2.1.318，2026-11-04。有限本地元数据检查未找到版本，不能认定未安装 |
| 3 安装 | 官方原生 Windows 安装与 WinGet 等入口，P0 不执行 |
| 4 插件 | 官方 Hook 配置可来自插件，也可直接用设置。插件包非强制前提 |
| 5 Hook | PreToolUse、PostToolUse、PostToolUseFailure、PermissionRequest 及子 Agent 等事件 |
| 6 生命周期 | pre 在工具 body 前，PermissionRequest 在需要询问的分支，post 区分成功/失败 |
| 7 前置阻止 | permissionDecision deny、适用事件 exit 2。使用同步 command/http，不用 async 做闸门 |
| 8 执行后 | PostToolUse / PostToolUseFailure，后置阻止不能撤回副作用 |
| 9 权限申请 | PermissionRequest 有 decision.behavior allow/deny 等协议。网络请求等特定路径有例外 |
| 10 接管审批 | 已公开该 Hook 的回答能力，但全访问、自动许可、无提示工具不会都触发权限事件 |
| 11 限制工具 | 原生 allow/ask/deny 与 PreToolUse 组合，拒绝优先规则按构建确认 |
| 12 子 Agent | SubagentStart/Stop、agent_id 等字段，skill/agent 局部 Hook 生命周期须测试 |
| 13 MCP | 工具名 matcher 可匹配 MCP。资源、认证、启动及服务内部调用另行建覆盖表 |
| 14 绕过路径 | 工具内部子进程、直接插件副作用、未匹配事件、后台 Hook、原生自动许可路径 |
| 15 超时 | 当前文档多数 command/http/mcp_tool 默认 600 秒，特定事件另有预算。默认 timeout 通常不阻止 pre |
| 16 异常 | command/http 的 onFailure block 在 v2.1.295+ 可将启动失败、非标准错误、超时和无效 JSON 变为阻止，须验证空输出等不构成错误的情形 |
| 17 配置 | ~/.claude/settings.json、项目 .claude/settings.json、settings.local.json、managed 与插件/skill/agent 作用域 |
| 18 日志历史 | Hook transcript_path，文档示例 ~/.claude/projects/.../*.jsonl。实际 Windows 路径未读取 |
| 19 更新 | onFailure 有最低版本，不能把旧 60 秒 Hook 博文作为当前默认值 |
| 20 卸载回滚 | 官方 /hooks 和插件管理可禁用/移除，未来适配仅修改自有条目并新会话验收 |

“PermissionDecision”按 permissionDecision 或 PermissionRequest 的 decision.behavior 处理，未发现需要命名为 PermissionDecision 的独立 Hook 事件依据。[S07](https://code.claude.com/docs/en/hooks) [S08](https://code.claude.com/docs/en/settings) [S09](https://code.claude.com/docs/en/setup) [S10](https://github.com/anthropics/claude-code/releases/tag/v2.1.318)

## A03 · DeepSeek Harness（DSH）

| 调查项 | 结果、接口与局限 |
|---|---|
| 1 官方身份 | deepseek-ai/deepseek-harness，非第三方同名站点 |
| 2 当前版本/获取 | 本机 PE 主程序 0.2.1.0，卸载器 0.2.1-rc.2，内部 dsh runtime 未知。公开列表 v0.2.2-rc.1 是预发布 |
| 3 安装 | 官方桌面 bundle 或 npm CLI，桌面内核与外装 CLI 不可混用 |
| 4 插件 | Cordis 插件贡献 services、typed events、reversible effects。Host 插件可在沙箱外进程内执行 |
| 5 Hook | tools/pre-execute、tools/execute、tools/post-execute 及 tools/result，不假定与 Claude 原生字段同构 |
| 6 生命周期 | call logged → pre → approval（必要时）→ monotonic guards → execute → post → authoritative result |
| 7 前置阻止 | pre 可拒绝，ctx.tools.guard 提供不可被普通后续 allow 覆盖的单调拒绝点。guard 注册本身的失载仍是风险 |
| 8 执行后 | tools/result 与持久 tool/result。优先最终 outcome，不能只取可被 post 改写的中间值 |
| 9 权限申请 | ctx.approval 一次性询问，不可用/取消时拒绝 |
| 10 接管审批 | 可开发 plugin 绑定 approval seam/外部策略桥，但未证明现有桌面组合可直接替换其 provider |
| 11 限制工具 | scoped registry restrictions 与 guards，作用域、preset、executor 选择均影响实际效果 |
| 12 子 Agent | 本地与外部 backend、独立 preset。必须检查 guard 继承与外部 backend 边界 |
| 13 MCP | 工具进入公共 registry，发现/资源等路径分开统计 |
| 14 绕过路径 | 任意 Host 插件直接 fs/spawn/network、替换 executor、卸载 guard、外部 backend 及额外 preset |
| 15 超时 | 工具 timeoutMs 是协作式，不能硬杀同进程恶意插件。插件→外部 AgentGuard 的超时拒绝需自行设计并测 |
| 16 异常 | 管线图的 pre/guard 错误进入失败结果，但保护插件未被加载并无等价阻断保证 |
| 17 配置 | Harness home 下 profile/package.json、cordis.patch.yml，桌面 reserved profiles/desktop。实际 home 位置未本地核实 |
| 18 日志历史 | session 持久化有版本化 session.vN.jsonl[.zstd]，其具体目录须读版本专属 metadata，P0 未读会话 |
| 19 更新 | 固定 master 提交 fcc0ad4 与安装候选没有建立对应，生产适配需发布 tag 和 bundle digest |
| 20 卸载回滚 | 官方 Plugin Manager 支持 disable/remove，是否 HMR 决定何时生效。恢复 profile 需保留其他用户内容 |

PTC / Code Mode 的 run_code 和嵌套工具调用进入受管 pipeline，这是可用的接入点。嵌套工具拒绝可反映为 binding rejection。原始 JS 或恶意插件越过 registry 的 OS 操作仍需 OS 隔离覆盖。[S11–14](13-SOURCE-REGISTER.md) [管线](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/docs/tool-execution-pipeline.md)

shell 经 ctx.shell/ctx.subprocess，PTY、LSP、ACP 存在不同 consumer。插件在 Host 沙箱外运行和实验性安全状态是 P1 前的关键限制。[S15–20](13-SOURCE-REGISTER.md)

## A04 · 豆包 App 内的工作模式

| 调查项 | 结果、接口与局限 |
|---|---|
| 1 官方身份 | 用户最终指定豆包 App 内工作模式。按 Windows 目标暂定 Windows 豆包客户端内该模式，非独立豆包工作 |
| 2 当前版本/获取 | 未知。需目标客户端 About、签名、ProductVersion 与模式名称，有限本地元数据未找到 |
| 3 安装 | Windows 官方下载入口及包身份需与目标客户端绑定，P0 未安装 |
| 4 插件 | 搜到官方开放平台 Skill 页面但正文不可读，未证实目标构建插件契约 |
| 5 Hook | 未证实 |
| 6 生命周期 | 模型/本地/云端工具生命周期公开契约未证实 |
| 7 前置阻止 | 第三方接口未证实 |
| 8 执行后 | 结构化本地回调/日志 API 未证实 |
| 9 权限申请 | 本地原生权限与外部回调关系未证实 |
| 10 接管审批 | 未证实 |
| 11 限制工具 | 外部受控接口未证实，不能把 Skill 指令当控制点 |
| 12 子 Agent | 此目标构建未证实 |
| 13 MCP | 此目标构建未证实，连接器不自动等于开放 MCP |
| 14 绕过路径 | 接口未知，GUI 自动化、浏览器、远程云电脑都应列作待验证路径 |
| 15 超时 | 未证实 |
| 16 异常 | 未证实 |
| 17 配置 | 未证实，避免根据其他豆包产品猜测路径 |
| 18 日志历史 | 未证实，P0 不读取会话内容 |
| 19 更新 | 版本/功能灰度可能影响，作为待验证假设 |
| 20 卸载回滚 | AgentGuard 未有可验证的插件安装方案，卸载方法未知。P1 先解决官方接入与可回滚性 |

官方 /work 页面只取得“豆包工作”标题，开放平台页面为 JS 壳，没有取得支持本地阻止的正文。搜索摘要保留为线索。A04 保持七款范围内，初始为未接入，观察 PoC 也需有来源后才可启用。[S21](https://www.doubao.com/work) [S22](https://open.doubao.com/document/doubao-developer-guide/skill-integration-guide)

## A05 · Reasonix

| 调查项 | 结果、接口与局限 |
|---|---|
| 1 官方身份 | esengine 的 Reasonix Studio/CLI，官网与仓库互链，不能因仓库名含 DeepSeek 就归属 DeepSeek 官方 |
| 2 当前版本/获取 | 本机卸载元数据 Studio 2.56.0，公开 latest Studio 2.57.0。CLI/runtime 版本未核 |
| 3 安装 | Studio Windows installer 与 CLI 分发，各自确认版本 |
| 4 插件 | 声明式 hooks/MCP 等与 full-trust runtime Extension 分两类 |
| 5 Hook | 原生 PreToolUse/PostToolUse/PermissionRequest 等，另有 Extension Protocol v2 |
| 6 生命周期 | 原生工具 hook 在工具前后，Extension tool.before/after、permission.decision 分开 |
| 7 前置阻止 | 原生 exit 2 可阻止 PreToolUse，Extension blocking intercept 可 block |
| 8 执行后 | 原生 PostToolUse 包括 success/failure，Extension observation 不保证不丢 |
| 9 权限申请 | 原生 PermissionRequest 通知存在。Guide 明确不能回答原生提示 |
| 10 接管审批 | 原生 Hook 不支持回答。Extension permission.decision 可 allow/deny，但 full-trust allow 能覆盖 host deny |
| 11 限制工具 | host deny 规则、原生 pre、Extension 替换策略分别评估，AgentGuard 不能允许 Extension 把原生 deny 提升为 allow |
| 12 子 Agent | Guide 指定部分工具/模型/compact hook 覆盖子 Agent，不等于全部 lifecycle 都覆盖 |
| 13 MCP | 产品支持 MCP，原生工具 Hook 的命名/资源/启动覆盖待安装版本验证 |
| 14 绕过路径 | full access、OS 沙箱外 Extension、可修改配置/脚本、Host 内直接执行、外部子进程 |
| 15 超时 | 原生 pre 与 permission 默认 5000 ms，其他常见 30000 ms。pre timeout 阻止，permission timeout 只是告警。Extension 预算另由协议定义 |
| 16 异常 | 原生无法 spawn/脚本崩溃通常不阻止，畸形文件不加载 hooks。Extension required/slot owner 失败可能使操作失败 |
| 17 配置 | Windows %APPDATA%\reasonix\config.toml / settings.json，项目 .reasonix/settings.json 需 trust |
| 18 日志历史 | state root/sessions，默认 Reasonix home，可能被 state home 改写。仅记录文档路径，未读取 |
| 19 更新 | 官网有 v1 链接，仓库生成索引是 v2。必须精确匹配 installed manifest/protocol |
| 20 卸载回滚 | plugin 管理、禁用/移除及 idle reload。记录 generation 与自有条目恢复，运行中 turn 不强换 |

官网称 Windows 使用 restricted token+ACL+Job，固定 CONFIG_PATHS 又称 Windows shell 在 OS 沙箱外。此冲突阻止给出“Windows 隔离已验证”的结论。原生 Hook、兼容插件 Hook、Extension 的行为也不能相互替代。[S23–30](13-SOURCE-REGISTER.md) [Guide](https://github.com/esengine/DeepSeek-Reasonix/blob/f4f32cc7d3078f7031c7852fbf0b7de410958199/docs/GUIDE.md) [Extension](https://github.com/esengine/DeepSeek-Reasonix/blob/f4f32cc7d3078f7031c7852fbf0b7de410958199/docs/EXTENSION_PROTOCOL.md)

## A06 · WorkBuddy

| 调查项 | 结果、接口与局限 |
|---|---|
| 1 官方身份 | 腾讯 WorkBuddy 桌面产品，独立于 CodeBuddy Code CLI/IDE |
| 2 当前版本/获取 | 本地卸载记录 5.9.3、Tencent 发布者。官方日志对应 2026-10-27，另有 5.11.0 灰度 |
| 3 安装 | 桌面官方 Windows 分发，本次未安装 |
| 4 插件 | 桌面文档列 Skill、MCP、Hook、Agent、Rule |
| 5 Hook | Hook 类型存在，明确的桌面 payload/verdict/timeout 契约未证实 |
| 6 生命周期 | 原生审计/任务可展示，外部逐工具 lifecycle 未证实 |
| 7 前置阻止 | 原生安全中心有工具禁用等阻止能力，第三方同步阻止回调未证实 |
| 8 执行后 | 原生审计中心可导出，实时开放协议未证实 |
| 9 权限申请 | 产品原生确认存在，外部 permission event 未证实 |
| 10 接管审批 | 未证实 |
| 11 限制工具 | 原生工具/MCP 管控公开，外部动态修改接口未证实 |
| 12 子 Agent | 产品专家/多人多 Agent 功能有入口，第三方完整事件覆盖未知 |
| 13 MCP | 桌面插件文档明确支持，运行与资源覆盖仍未知 |
| 14 绕过路径 | 本地/云电脑、GUI、插件、全局 full access、后台子任务及未公开内部工具需测试 |
| 15 超时 | 桌面 Hook 未证实，拒绝采用 CodeBuddy Code 的值 |
| 16 异常 | 桌面 Hook 未证实 |
| 17 配置 | 文档提到 .codebuddy 开发配置的工作区信任，不能据此认定 CLI settings.json 对桌面生效 |
| 18 日志历史 | 原生审计导出入口已证实，稳定目录/机器格式未证实 |
| 19 更新 | 5.9.3 修复过 Windows sandbox 初始化问题，构建改变必须重新验收 |
| 20 卸载回滚 | 桌面插件管理页面支持卸载。AgentGuard 插件字段和回滚测试待契约确定 |

含“WorkBuddy Enterprise”分类、正文却描述 CodeBuddy Code 的腾讯云 Hook 文档不作为桌面接口证据。P1 仅可先做用户触发的原生审计导入或待验证观察方案。[S31](https://www.workbuddy.cn/docs/workbuddy/Plugins) [S32](https://www.workbuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Permission-Modes) [S33](https://www.workbuddy.cn/docs/workbuddy/Changelog)

## A07 · ZCode

| 调查项 | 结果、接口与局限 |
|---|---|
| 1 官方身份 | zcode.z.ai 的 ZCode / 第一方 ZCode Agent，其它内嵌 Agent 必须另核 |
| 2 当前版本/获取 | 文档滚动，无对应本地构建。后续 About 与签名元数据确认 |
| 3 安装 | 官方 Windows installer，P0 未安装 |
| 4 插件 | hooks/hooks.json 自动发现，启用状态控制加载 |
| 5 Hook | 同步 process 或 command，另可 async command |
| 6 生命周期 | pre → 必要时 permission → execute → success post / failure post |
| 7 前置阻止 | permissionDecision deny、适用事件 exit 2，deny > ask > allow |
| 8 执行后 | tool_response、PostToolUseFailure 的 error，临时 transcript 非永久存储 |
| 9 权限申请 | PermissionRequest 在需要确认分支，可返回 behavior、updatedInput、updatedPermissions |
| 10 接管审批 | 有公开 JSON 返回契约，仍受 explicit deny/Plan/hard limits 约束 |
| 11 限制工具 | 原生 deny/hard limit 不能被 hook allow 覆盖 |
| 12 子 Agent | 产品支持子 Agent，未证实所有子工具完整继承 Hook |
| 13 MCP | 官方 MCP 页面，动态工具名与所有非 tool 方法覆盖待核 |
| 14 绕过路径 | async、未启用用户 Hook、忽略的项目 Hook、其它内嵌 Agent、工具 body 及远程副作用 |
| 15 超时 | timeoutMs 毫秒优先于 timeout 秒，默认 60000 ms，超时后的最终 verdict 仍需验证 |
| 16 异常 | 其它非零码/错误字段可恢复失败，后续 Hook 可继续，不据此推定所有异常一定阻止 |
| 17 配置 | ~/.zcode/cli/config.json 需 hooks.enabled=true。项目 .zcode/config.json/zcode.json 的 Hook 当前被忽略 |
| 18 日志历史 | ~/.zcode/cli/log/zcode-<date>.jsonl，transcript_path 为本次 Hook 临时文件，完整长期会话位置未知 |
| 19 更新 | 文档配置段称项目 Hook 忽略，设置操作段仍提 workspace scope，存在表述漂移 |
| 20 卸载回滚 | 用户设置条目可删除/停用，插件随 enable 管理。新会话捕获配置，P1 需实际验证 |

用户/项目权限规则与用户/项目 Hook 是两个维度。原生安全页的项目授权选项不能证明项目 Hook 可执行。具体语义冲突以固定构建实测解决。[S35](https://zcode.z.ai/en/docs/hooks) [S36–39](13-SOURCE-REGISTER.md)

## 本地版本元数据记录

只读取注册的卸载信息、Appx 身份、限定应用目录中的 PE VersionInfo。没有读取配置、会话、token 或凭证，没有执行版本命令。签名加密校验未完成，Publisher 字段不能替代 Authenticode 验证。

| 对象 | 观测值 | 可得结论 |
|---|---|---|
| OpenAI.Codex | Appx 26.1029.7803.0，publisher 为包 DN | 包存在，内核未知 |
| Reasonix Studio | 2.56.0，Publisher esengine | 卸载记录版本，不代表运行实例 |
| WorkBuddy | 5.9.3，Publisher Tencent Technology (Shenzhen) Company Limited | 区分于 CodeBuddy |
| CodeBuddy (User) | 1.106.1 | 仅用于产品消歧，不纳入兼容范围 |
| DeepSeek Harness.exe | ProductVersion 0.2.1.0 | 内部 runtime/rc 映射未知 |
| Uninstall DeepSeek Harness.exe | 0.2.1-rc.2 | 与主程序元数据不同，保留差异 |
| Windows | OsName Windows 11 家庭版 中文版，10.0.26200 | 目标家庭版适用，补丁修订号/MXC 未核 |

## 技术依据

逐项结论依据上文各产品官方链接与来源登记。E1/E2只支持接口存在及静态行为说明，本机元数据只支持安装身份，没有E3/E4运行证据。出现正文不可读、协议冲突或release未映射时保留未证实。[来源登记](13-SOURCE-REGISTER.md)

## 风险

同名 Hook 并不等价，同一产品的滚动文档也可能超前安装版本。原生 UI 可拒绝操作不能推导出外部可接管。对未证实产品强行适配会产生虚假的绿色保护状态。

## 尚未确定的内容

A04 具体 Windows 客户端和版本，七款精确运行内核与签名，WorkBuddy 桌面同步 Hook 契约，ZCode 超时、子 Agent 继承，Reasonix 协议/沙箱冲突，以及每款旁路工具清单。均映射到开放问题与 P1 gate。[开放问题](12-OPEN-QUESTIONS.md) [测试计划](10-TEST-AND-ACCEPTANCE-PLAN.md)


## P0.1执行模式与证据冻结

保留以上七款各20项调查及原E0–E4分级。本轮无任何Agent联调，当前全部L0，五项能力全部false。A入口证据与B委托执行是不同合同，七款B均未验证，C也均未取得AgentGuard认证。A04用户已确定豆包 App 内工作模式，Windows客户端具体包、构建与接口仍未证实。

Codex E1、Claude Code E1、DSH E2、豆包E0、Reasonix E2、WorkBuddy E1、ZCode E1不因本轮Schema或向量验证提升。WorkBuddy不是CodeBuddy，Reasonix原生Hook与Extension v2分开，DSH guard不是OS边界，ZCode项目Hook不可当可部署接口。原文“已证实”仅指对应来源层的事实。

L2使用入口显式deny的E3认证，故障继续限制如实披露。L3独立要求E4 OS证据，不把公开接口计为运行认证。[能力定义](16-PROTECTION-CAPABILITY-CONTRACT.md) [三模式合同](15-EXECUTION-BROKER-CONTRACT.md)

