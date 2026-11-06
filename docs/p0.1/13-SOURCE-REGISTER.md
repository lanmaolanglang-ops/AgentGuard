# 来源登记与证据边界

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05

## 历史来源适用范围

下方R/L/S01–S58保留P0原调研记录，段落中的“本次”指P0原调查，不能作为P0.1重新运行或最新构建证明。P0.1没有更新本地Agent元数据。新增官方资料与逐链接可达性检查列于末尾，HTTP200不代表正文完整、接口匹配或E3/E4。

## 目的

使重要能力结论能追溯到实际打开的官方资料、固定提交或有限本地元数据，记录不可读与冲突资料，防止搜索摘要代替接口证据。

## 结论

本次已联网，使用官方文档及公开仓库正文，读取了 DSH ToolsService 的实际源码片段。多数接口结论仍属于文档或静态证据，没有真实 Agent 联调。58 条登记项包含可用证据与明确排除的检索入口，数量不代表验证覆盖。

## 技术依据

E0 为正文不可读/未确认。E1 为已读官方文档/Release。E2 为固定官方仓库的文档或源码静态证据。E3 为精确环境可复现实验，E4 为 OS 隔离边界负面测试。E2 条目逐项注明是否实际源码，不把固定提交文档称为已完成代码审计。本文 E3/E4 均为空。

URL 后的版本说明只对相应资料成立。滚动文档抓取日期不能等同安装版本，也不能把分支 HEAD 当 release。不同来源有冲突时记入开放问题，不挑选更乐观结论。

本次通过公开 git ls-remote 取得 DSH master 和 Reasonix studio 完整提交 SHA，读取公开原始文件用于核对。未克隆或执行仓库代码，未发送模型/API业务请求。

## 用户需求和本地证据

### R01 · 用户 P0 需求及更正

输入附件为本次粘贴的技术预研任务，项目版本 V2.0，目标 Windows 11，七款 Agent，全交付位于 D:\AgentGuard\docs\p0。用户先答独立豆包工作，随后更正为“应该是豆包APP里的工作模式”。以最后一次更正为准，Windows 客户端构建仍未知。依据为当前对话，可信度为直接需求。

### L01 · 工作区与元数据

检查时 D:\AgentGuard 为空，git status 返回非 Git 仓库，未发现父目录/工作区现有 AGENTS.md 文件。当前聊天中提供的 Windows GUI 自动化规则照常适用。文件写入均为新增 P0 Markdown。

只读查询了匹配产品的卸载 DisplayName/DisplayVersion/Publisher/InstallLocation、OpenAI.Codex Appx metadata、限定软件目录下的 PE VersionInfo 与 OS metadata。结果如下。

| 产品 | 元数据 |
|---|---|
| Codex | OpenAI.Codex，26.1029.7803.0 |
| Reasonix Studio | 2.56.0，esengine |
| WorkBuddy | 5.9.3，Tencent Technology (Shenzhen) Company Limited |
| CodeBuddy (User) | 1.106.1，仅消歧，非新增兼容目标 |
| DeepSeek Harness 主程序 | ProductVersion 0.2.1.0 |
| DeepSeek Harness 卸载器 | ProductVersion 0.2.1-rc.2 |
| OS | Windows 11 家庭版 中文版，10.0.26200 |

未找到的产品只表示此次有限元数据检查无记录，不等于未安装。Publisher 是元数据，未完成 Authenticode/包签名链验证。未读取任何 Agent 配置、会话内容、认证文件或环境中的凭证。

## 官方资料

### S01 · OpenAI Hooks

- 来源名称为 OpenAI Hooks。
- 对应版本或提交为 滚动文档，2026-11-05 抓取，未绑定本机内核版本。
- 接口说明与结论为 PreToolUse 可 deny，PermissionRequest 可 allow/deny，PostToolUse 提供结果。Tool coverage 排除 hosted tools，write_stdin 不重复前置检查，错误/超时可放行。大多数 Hook 默认 600 秒，Interrupt/SessionEnd 有较短预算。
- 可信度为 E1 高，部署适配未证实。
- 官方链接见 [OpenAI Hooks](https://learn.chatgpt.com/docs/hooks)。

### S02 · OpenAI Agent approvals & security

- 来源名称为 OpenAI Agent approvals & security。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 审批策略与沙箱分离。on-request 和 never 含义不同，支持 granular。显式 selectable untrusted 已退役，不能沿用旧配置假设。
- 可信度为 E1 高。
- 官方链接见 [OpenAI Agent approvals & security](https://learn.chatgpt.com/docs/agent-approvals-security)。

### S03 · OpenAI Windows sandbox

- 来源名称为 OpenAI Windows sandbox。
- 对应版本或提交为 滚动文档，提及 CLI 0.162.0+ 的 MXC 检查入口。
- 接口说明与结论为 mxc 依赖主机能力，elevated 需管理员批准的初始化，unelevated 网络隔离较弱。MXC 不可用时显式选择应失败。主机版本不能替代隔离验证。
- 可信度为 E1 高，设备适用性未知。
- 官方链接见 [OpenAI Windows sandbox](https://learn.chatgpt.com/docs/windows/windows-sandbox)。

### S04 · OpenAI Codex App Server

- 来源名称为 OpenAI Codex App Server。
- 对应版本或提交为 当前 v2 协议文档，具体 runtime 未绑定。
- 接口说明与结论为 commandExecution/fileChange/requestApproval 与 item/completed 分离。客户端可以回应自己所连接实例的审批。网络审批可能聚合同目的地请求。
- 可信度为 E1 高，接管既有桌面会话未证实。
- 官方链接见 [OpenAI Codex App Server](https://learn.chatgpt.com/docs/app-server)。

### S05 · OpenAI Package your plugin

- 来源名称为 OpenAI Package your plugin。
- 对应版本或提交为 当前 portable/compatibility manifest 文档。
- 接口说明与结论为 插件可携带 hooks/hooks.json。安装不自动信任 Hook，需审核定义。支持面向桌面的手动分发，不能假定公开插件目录接收生命周期 Hook。
- 可信度为 E1 高。
- 官方链接见 [OpenAI Package your plugin](https://developers.openai.com/plugins/build/plugins)。

### S06 · OpenAI Advanced Configuration

- 来源名称为 OpenAI Advanced Configuration。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 用户与项目 .codex/hooks.json 或 config.toml 内联 Hook，项目配置需要信任。仅记录官方路径，本次未读取这些本地文件。
- 可信度为 E1 高。
- 官方链接见 [OpenAI Advanced Configuration](https://learn.chatgpt.com/docs/config-file/config-advanced)。

### S07 · Anthropic Hooks reference

- 来源名称为 Anthropic Hooks reference。
- 对应版本或提交为 滚动文档，onFailure 要求 v2.1.295+。
- 接口说明与结论为 PreToolUse 支持 permissionDecision，PermissionRequest 使用 decision.behavior。PermissionDecision 是决策字段概念，未确认为独立事件名。command/http 可用 onFailure=block，默认错误/超时通常回原生权限流。
- 可信度为 E1 高，需构建级验证。
- 官方链接见 [Anthropic Hooks reference](https://code.claude.com/docs/en/hooks)。

### S08 · Anthropic Settings files and precedence

- 来源名称为 Anthropic Settings files and precedence。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 用户 settings.json、项目 settings.json/local、managed 配置有不同作用域及合并行为。
- 可信度为 E1 高。
- 官方链接见 [Anthropic Settings files and precedence](https://code.claude.com/docs/en/settings)。

### S09 · Anthropic Advanced setup

- 来源名称为 Anthropic Advanced setup。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 Windows 原生安装、版本管理、卸载流程公开。本文仅调查，未执行安装或版本命令。
- 可信度为 E1 高。
- 官方链接见 [Anthropic Advanced setup](https://code.claude.com/docs/en/setup)。

### S10 · Anthropic Release v2.1.318

- 来源名称为 Anthropic Release v2.1.318。
- 对应版本或提交为 v2.1.318，页面发布日期 2026-11-04。
- 接口说明与结论为 latest 在调研时重定向到本 tag。发布号不能证明本机已安装此版本。
- 可信度为 E1 高。
- 官方链接见 [Anthropic Release v2.1.318](https://github.com/anthropics/claude-code/releases/tag/v2.1.318)。

### S11 · DeepSeek Harness Architecture

- 来源名称为 DeepSeek Harness Architecture。
- 对应版本或提交为 master 固定提交 fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159。
- 接口说明与结论为 Cordis 可替换服务与插件树，工具 pre/execute/post waterfall，桌面携带自己的 Host/runtime，profile 配置与 session 持久事件分离。
- 可信度为 E2 中高，非安装版本。
- 官方链接见 [DeepSeek Harness Architecture](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/docs/architecture.md)。

### S12 · DSH Tools subsystem

- 来源名称为 DSH Tools subsystem。
- 对应版本或提交为 同 S11 提交。
- 接口说明与结论为 工具注册、守卫及执行上下文公开。同进程代码的超时依赖协作取消，不能保证硬杀死。
- 可信度为 E2 中高。
- 官方链接见 [DSH Tools subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/docs/subsystems/tools.md)。

### S13 · DSH Tool Execution Pipeline

- 来源名称为 DSH Tool Execution Pipeline。
- 对应版本或提交为 同 S11 提交。
- 接口说明与结论为 pre-execute 后检查 monotonic guards，拒绝跳过 body。ctx.approval 不可答时拒绝。PTC run_code 与嵌套调用过管线。
- 可信度为 E2 中高。
- 官方链接见 [DSH Tool Execution Pipeline](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/docs/tool-execution-pipeline.md)。

### S14 · DSH ToolsService source

- 来源名称为 DSH ToolsService source。
- 对应版本或提交为 同 S11 提交。
- 接口说明与结论为 已阅读 guard()/checkGuards() 与 execute() 片段。全局及继承守卫被遍历，execute 委托选定 executor。源文件不证明恶意插件无法直接调用 OS。
- 可信度为 E2 高，覆盖结论有限。
- 官方链接见 [DSH ToolsService source](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/packages/core/tools/src/index.ts)。

### S15 · DSH MCP subsystem

- 来源名称为 DSH MCP subsystem。
- 对应版本或提交为 同 S11 提交。
- 接口说明与结论为 MCP 工具映射普通工具，配置 stdio/HTTP。资源读取按作用域，存在独立资源和发现路径。
- 可信度为 E2 中高。
- 官方链接见 [DSH MCP subsystem](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/docs/subsystems/mcp.md)。

### S16 · DSH Subagents

- 来源名称为 DSH Subagents。
- 对应版本或提交为 同 S11 提交。
- 接口说明与结论为 支持本地与外部执行、独立 preset，子会话目录及血缘。外部 backend 的权限不能从父代理自动推定。
- 可信度为 E2 中高。
- 官方链接见 [DSH Subagents](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/docs/subsystems/subagent.md)。

### S17 · DSH Subprocess

- 来源名称为 DSH Subprocess。
- 对应版本或提交为 同 S11 提交。
- 接口说明与结论为 shell/PTY/LSP/ACP 使用 subprocess seam，是受管子进程的接入候选。插件直接 Node OS 调用仍需另行处理。
- 可信度为 E2 中高。
- 官方链接见 [DSH Subprocess](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/docs/subsystems/subprocess.md)。

### S18 · DSH Plugin Manager

- 来源名称为 DSH Plugin Manager。
- 对应版本或提交为 同 S11 提交。
- 接口说明与结论为 支持启停及安装移除。Host 插件在进程内、工作区沙箱外执行，HMR 改变生效时机。
- 可信度为 E2 高。
- 官方链接见 [DSH Plugin Manager](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/packages/boot/plugin-manager/README.md)。

### S19 · DSH Safety

- 来源名称为 DSH Safety。
- 对应版本或提交为 同 S11 提交。
- 接口说明与结论为 明确处于实验开发预览、未经过安全审计，不能用作不可信负载的唯一安全控制。
- 可信度为 E2 高。
- 官方链接见 [DSH Safety](https://github.com/deepseek-ai/deepseek-harness/blob/fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159/SAFETY.md)。

### S20 · DSH Releases

- 来源名称为 DSH Releases。
- 对应版本或提交为 2026-11-05，列表首项 v0.2.2-rc.1，2026-11-04。
- 接口说明与结论为 候选版并非稳定版，本机 PE 元数据与发布 tag 未完成映射。
- 可信度为 E1 高，版本对应未知。
- 官方链接见 [DSH Releases](https://github.com/deepseek-ai/deepseek-harness/releases)。

### S21 · 豆包官方客户端入口

- 来源名称为 豆包官方客户端入口。
- 对应版本或提交为 2026-11-05，原生 HTTP 200，仅读到标题。
- 接口说明与结论为 标题是豆包工作。用户最终指定豆包 App 内工作模式，因此此入口作为产品消歧资料，不能建立目标客户端 Hook 能力。
- 可信度为 E0，正文不可读。
- 官方链接见 [豆包官方客户端入口](https://www.doubao.com/work)。

### S22 · 豆包开放平台 Skill integration

- 来源名称为 豆包开放平台 Skill integration。
- 对应版本或提交为 2026-11-05，HTTP 200，JS 页面无可读正文。
- 接口说明与结论为 仅确认官方开发文档入口。搜索摘要提及 Skill，但未用摘要认定接口，具体客户端、Skill 与本地审批契约均未证实。
- 可信度为 E0，不能支持拦截结论。
- 官方链接见 [豆包开放平台 Skill integration](https://open.doubao.com/document/doubao-developer-guide/skill-integration-guide)。

### S23 · Reasonix Docs

- 来源名称为 Reasonix Docs。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 官网说明 Windows restricted token+ACL+Job 的受限预设，且部分扩展链接仍称 v1。与固定仓库说明有冲突。
- 可信度为 E1 中，冲突未解决。
- 官方链接见 [Reasonix Docs](https://reasonix.io/docs/)。

### S24 · Reasonix Guide

- 来源名称为 Reasonix Guide。
- 对应版本或提交为 studio 固定提交 f4f32cc7d3078f7031c7852fbf0b7de410958199。
- 接口说明与结论为 原生 Hook exit 2 阻止 PreToolUse，timeout 阻止，但启动失败/崩溃通常只警告。原生 PermissionRequest 不能答审批。全局及受信项目 Hook，子 Agent 有部分事件覆盖。
- 可信度为 E2 中高，版本映射未知。
- 官方链接见 [Reasonix Guide](https://github.com/esengine/DeepSeek-Reasonix/blob/f4f32cc7d3078f7031c7852fbf0b7de410958199/docs/GUIDE.md)。

### S25 · Reasonix Extensions

- 来源名称为 Reasonix Extensions。
- 对应版本或提交为 同 S24 提交。
- 接口说明与结论为 Manifest v2 runtime 是 full trust，位于沙箱外，可越过工作区权限。permission.decision allow 甚至覆盖 host deny。required/slot owner 影响失败语义。
- 可信度为 E2 高。
- 官方链接见 [Reasonix Extensions](https://github.com/esengine/DeepSeek-Reasonix/blob/f4f32cc7d3078f7031c7852fbf0b7de410958199/docs/EXTENSIONS.md)。

### S26 · Reasonix Extension Protocol

- 来源名称为 Reasonix Extension Protocol。
- 对应版本或提交为 reasonix.extension.v2，同 S24 提交。
- 接口说明与结论为 stdio NDJSON JSON-RPC，blocking intercept 与非阻塞 observation 分开，tool.before、permission.decision 等契约，观测队列饱和可丢事件。
- 可信度为 E2 高。
- 官方链接见 [Reasonix Extension Protocol](https://github.com/esengine/DeepSeek-Reasonix/blob/f4f32cc7d3078f7031c7852fbf0b7de410958199/docs/EXTENSION_PROTOCOL.md)。

### S27 · Reasonix Generated protocol index

- 来源名称为 Reasonix Generated protocol index。
- 对应版本或提交为 v2，schema hash a0716fcfa8aab0f67a5c2299c4e6fa1573378e44c6050633a930d6d736f35fd7。
- 接口说明与结论为 生成索引确认 v2 与 extension/intercept、extension/event。旧 v1 说明不能用于本版直接适配。
- 可信度为 E2 高。
- 官方链接见 [Reasonix Generated protocol index](https://github.com/esengine/DeepSeek-Reasonix/blob/f4f32cc7d3078f7031c7852fbf0b7de410958199/docs/EXTENSION_PROTOCOL.generated.md)。

### S28 · Reasonix Configuration Paths

- 来源名称为 Reasonix Configuration Paths。
- 对应版本或提交为 同 S24 提交。
- 接口说明与结论为 Windows home=%APPDATA%\reasonix，settings.json、config.toml、sessions 路径。文件另称 Windows shell 无 OS 沙箱，与官网冲突。
- 可信度为 E2 中，隔离结论未知。
- 官方链接见 [Reasonix Configuration Paths](https://github.com/esengine/DeepSeek-Reasonix/blob/f4f32cc7d3078f7031c7852fbf0b7de410958199/docs/CONFIG_PATHS.md)。

### S29 · Reasonix Plugin Packages

- 来源名称为 Reasonix Plugin Packages。
- 对应版本或提交为 同 S24 提交。
- 接口说明与结论为 插件管理与复制内容摘要公开。文中部分兼容 Hook 描述与 Guide 原生 Hook 不同，未做 installed mapping。
- 可信度为 E2 中高。
- 官方链接见 [Reasonix Plugin Packages](https://github.com/esengine/DeepSeek-Reasonix/blob/f4f32cc7d3078f7031c7852fbf0b7de410958199/docs/PLUGIN_PACKAGES.md)。

### S30 · Reasonix Studio Release

- 来源名称为 Reasonix Studio Release。
- 对应版本或提交为 studio-v2.57.0，2026-11-02。
- 接口说明与结论为 latest 指向该版，本地 uninstall 记录 2.56.0，固定 studio HEAD 还比 release 更新。
- 可信度为 E1 高。
- 官方链接见 [Reasonix Studio Release](https://github.com/esengine/DeepSeek-Reasonix/releases/tag/studio-v2.57.0)。

### S31 · WorkBuddy 桌面插件系统

- 来源名称为 WorkBuddy 桌面插件系统。
- 对应版本或提交为 滚动文档，2026-11-05，未绑定 5.9.3。
- 接口说明与结论为 桌面文档直接列 Skill/MCP/Hook/Agent/Rule，管理页面可卸载。没有给出可依赖的前置阻止/审批返回协议。
- 可信度为 E1 高，阻止未证实。
- 官方链接见 [WorkBuddy 桌面插件系统](https://www.workbuddy.cn/docs/workbuddy/Plugins)。

### S32 · WorkBuddy 默认权限与安全沙箱

- 来源名称为 WorkBuddy 默认权限与安全沙箱。
- 对应版本或提交为 滚动文档，未绑定 5.9.3。
- 接口说明与结论为 原生文件/命令/网络和工具管控可拒绝，审计可导出。完全访问开关是全局。未公开 AgentGuard 动态接管该审批的契约。
- 可信度为 E1 高。
- 官方链接见 [WorkBuddy 默认权限与安全沙箱](https://www.workbuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Permission-Modes)。

### S33 · WorkBuddy 更新日志

- 来源名称为 WorkBuddy 更新日志。
- 对应版本或提交为 5.9.3=2026-10-27，5.11.0=2026-11-03 灰度。
- 接口说明与结论为 本地 5.9.3 可对应发布记录，5.11.0 灰度不能当作所有设备当前版本。
- 可信度为 E1 高。
- 官方链接见 [WorkBuddy 更新日志](https://www.workbuddy.cn/docs/workbuddy/Changelog)。

### S34 · WorkBuddy 数据管理

- 来源名称为 WorkBuddy 数据管理。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 提供用户数据管理入口。没有据此确认稳定本地逐工具事件格式或存储路径。
- 可信度为 E1 中，机器可读审计未知。
- 官方链接见 [WorkBuddy 数据管理](https://www.workbuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Data)。

### S35 · ZCode Hooks

- 来源名称为 ZCode Hooks。
- 对应版本或提交为 滚动文档，未提供本次对应二进制版本。
- 接口说明与结论为 前置与权限返回接口公开。用户 hooks.enabled，项目级 Hook 被忽略，插件可分发。非零错误可恢复，超时最终放行/阻止未充分说明。
- 可信度为 E1 高，部署适配未证实。
- 官方链接见 [ZCode Hooks](https://zcode.z.ai/en/docs/hooks)。

### S36 · ZCode Install

- 来源名称为 ZCode Install。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 Windows 安装入口公开，未核得本地或唯一最新构建。官网中的关闭防护建议不纳入本项目安装方案。
- 可信度为 E1 中高。
- 官方链接见 [ZCode Install](https://zcode.z.ai/en/docs/install)。

### S37 · ZCode Safety Confirmation

- 来源名称为 ZCode Safety Confirmation。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 原生权限有项目/会话等选项，普通问题可超时继续，但权限和计划确认持续等待。二者不能混淆。
- 可信度为 E1 高。
- 官方链接见 [ZCode Safety Confirmation](https://zcode.z.ai/en/docs/safety-confirm)。

### S38 · ZCode Subagents

- 来源名称为 ZCode Subagents。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 有子 Agent 产品能力，不能据此认定全部子执行均触发同一 Hook。
- 可信度为 E1 中高。
- 官方链接见 [ZCode Subagents](https://zcode.z.ai/en/docs/subagents)。

### S39 · ZCode MCP

- 来源名称为 ZCode MCP。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 提供 MCP 接入说明，覆盖范围还需匹配工具名和构建验证。
- 可信度为 E1 中高。
- 官方链接见 [ZCode MCP](https://zcode.z.ai/en/docs/mcp-services)。

### S40 · Microsoft Restricted Tokens

- 来源名称为 Microsoft Restricted Tokens。
- 对应版本或提交为 Win32 文档，2026-11-05。
- 接口说明与结论为 限制 SID/特权，普通应用可用调用者主令牌的 restricted 版本创建受限进程，仍受具体 API 权限约束。
- 可信度为 E1 高。
- 官方链接见 [Microsoft Restricted Tokens](https://learn.microsoft.com/en-us/windows/win32/secauthz/restricted-tokens)。

### S41 · Microsoft Job Objects

- 来源名称为 Microsoft Job Objects。
- 对应版本或提交为 Win32 文档，2026-11-05。
- 接口说明与结论为 进程组生命周期、资源与嵌套 Job。它本身不提供文件/网络逐操作审批。
- 可信度为 E1 高。
- 官方链接见 [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)。

### S42 · Microsoft Launch an AppContainer

- 来源名称为 Microsoft Launch an AppContainer。
- 对应版本或提交为 Windows 8+ API，2026-11-05。
- 接口说明与结论为 AppContainer SID/capability 与 DACL 共同约束进程资源访问，兼容性和 broker 需独立设计。
- 可信度为 E1 高。
- 官方链接见 [Microsoft Launch an AppContainer](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer)。

### S43 · Microsoft Windows Sandbox

- 来源名称为 Microsoft Windows Sandbox。
- 对应版本或提交为 Windows 10/11 产品文档。
- 接口说明与结论为 Home 不支持该可选桌面虚拟化产品。不能由此推定所有 Win32 隔离 API 在 Home 不可用。
- 可信度为 E1 高。
- 官方链接见 [Microsoft Windows Sandbox](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/)。

### S44 · Microsoft ACL

- 来源名称为 Microsoft ACL。
- 对应版本或提交为 Win32 文档。
- 接口说明与结论为 DACL 按 trustee 访问控制，SACL 为审计。相同用户身份的进程不会天然分开。
- 可信度为 E1 高。
- 官方链接见 [Microsoft ACL](https://learn.microsoft.com/en-us/windows/win32/secauthz/access-control-lists)。

### S45 · Microsoft Named Pipe Security

- 来源名称为 Microsoft Named Pipe Security。
- 对应版本或提交为 Win32 文档。
- 接口说明与结论为 默认 pipe DACL 可能对 Everyone/anonymous 开放读取，必须指定安全描述符。
- 可信度为 E1 高。
- 官方链接见 [Microsoft Named Pipe Security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)。

### S46 · Microsoft ETW sessions

- 来源名称为 Microsoft ETW sessions。
- 对应版本或提交为 Win32 文档。
- 接口说明与结论为 跨进程 session 控制通常要求 elevated、Performance Log Users 或相应服务身份。
- 可信度为 E1 高。
- 官方链接见 [Microsoft ETW sessions](https://learn.microsoft.com/en-us/windows/win32/etw/controlling-event-tracing-sessions)。

### S47 · Microsoft StartTrace

- 来源名称为 Microsoft StartTrace。
- 对应版本或提交为 Win32 文档。
- 接口说明与结论为 NT Kernel Logger 权限更严格，in-process private trace 与整机内核事件能力不同。
- 可信度为 E1 高。
- 官方链接见 [Microsoft StartTrace](https://learn.microsoft.com/en-us/windows/win32/api/evntrace/nf-evntrace-starttracea)。

### S48 · Microsoft Sysmon

- 来源名称为 Microsoft Sysmon。
- 对应版本或提交为 页面 v15.22，2026-09-10。
- 接口说明与结论为 Sysmon 由 service 与 device driver 组成，可辅助系统事件关联，不能作为默认无安装路线。
- 可信度为 E1 高。
- 官方链接见 [Microsoft Sysmon](https://learn.microsoft.com/en-us/sysinternals/downloads/sysmon)。

### S49 · Microsoft Controlled Folder Access

- 来源名称为 Microsoft Controlled Folder Access。
- 对应版本或提交为 2026-11-05 文档。
- 接口说明与结论为 Home 可使用带 Defender 的 CFA，依赖 active AV/实时防护，按 app 信任控制文件修改，不能表达会话级授权。
- 可信度为 E1 高。
- 官方链接见 [Microsoft Controlled Folder Access](https://learn.microsoft.com/en-us/defender-endpoint/controlled-folder-access-overview)。

### S50 · Microsoft Hyper-V hardware requirements

- 来源名称为 Microsoft Hyper-V hardware requirements。
- 对应版本或提交为 主机要求文档，2026-11-05。
- 接口说明与结论为 虚拟化存在硬件与平台前提。家庭版兼容 hypervisor/产品/许可组合需另作 P1 实验，未确定 VM 产品。
- 可信度为 E1 中高。
- 官方链接见 [Microsoft Hyper-V hardware requirements](https://learn.microsoft.com/en-us/windows-server/virtualization/hyper-v/host-hardware-requirements)。

### S51 · Tauri 2 Capabilities

- 来源名称为 Tauri 2 Capabilities。
- 对应版本或提交为 Tauri 2 滚动文档。
- 接口说明与结论为 限制窗口/前端可用 commands，不能赋予第三方 Agent 操作系统安全边界。
- 可信度为 E1 高。
- 官方链接见 [Tauri 2 Capabilities](https://v2.tauri.app/security/capabilities/)。

### S52 · Tauri Process Model

- 来源名称为 Tauri Process Model。
- 对应版本或提交为 Tauri 2 滚动文档。
- 接口说明与结论为 Core/WebView 多进程，Windows 使用 WebView2。UI/Core 进程架构需另加独立策略进程以隔离 UI 生命周期。
- 可信度为 E1 高。
- 官方链接见 [Tauri Process Model](https://v2.tauri.app/concept/process-model/)。

### S53 · Electron Security

- 来源名称为 Electron Security。
- 对应版本或提交为 latest 文档，2026-11-05。
- 接口说明与结论为 contextIsolation、sandbox、关闭 nodeIntegration、IPC 校验是安全设计要求。
- 可信度为 E1 高。
- 官方链接见 [Electron Security](https://www.electronjs.org/docs/latest/tutorial/security)。

### S54 · SQLite WAL

- 来源名称为 SQLite WAL。
- 对应版本或提交为 2026-11-05 滚动文档。
- 接口说明与结论为 WAL 读写可并行但同一时刻单 writer，存在 busy/checkpoint/本地共享内存约束。
- 可信度为 E1 高。
- 官方链接见 [SQLite WAL](https://sqlite.org/wal.html)。

### S55 · SQLite Appropriate Uses

- 来源名称为 SQLite Appropriate Uses。
- 对应版本或提交为 2026-11-05 滚动文档。
- 接口说明与结论为 嵌入式本地存储适用，不能据此承诺未测吞吐。
- 可信度为 E1 高。
- 官方链接见 [SQLite Appropriate Uses](https://sqlite.org/whentouse.html)。

### S56 · Rust official overview

- 来源名称为 Rust official overview。
- 对应版本或提交为 2026-11-05。
- 接口说明与结论为 适于底层与 CLI。内存安全收益属于语言选择依据，FFI unsafe 与安全策略正确性仍需审核。
- 可信度为 E1 中高。
- 官方链接见 [Rust official overview](https://www.rust-lang.org/what/)。

### S57 · OpenAI Permissions profiles

- 来源名称为 OpenAI Permissions profiles。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 新 permission profiles 与旧 sandbox_mode 配置应择一，workspace/read-only/full-access 等含义按版本核对。
- 可信度为 E1 高。
- 官方链接见 [OpenAI Permissions profiles](https://learn.chatgpt.com/docs/permissions)。

### S58 · OpenAI Configuration Reference

- 来源名称为 OpenAI Configuration Reference。
- 对应版本或提交为 滚动文档，2026-11-05。
- 接口说明与结论为 审批策略、历史持久化与版本敏感字段的参考。文档无法证明本机加载有效值。
- 可信度为 E1 高。
- 官方链接见 [OpenAI Configuration Reference](https://learn.chatgpt.com/docs/config-file/config-reference)。

## 抓取限制、冲突与未采用来源

| 项目 | 处理 |
|---|---|
| OpenAI 旧 developers 路径部分 404 | 改查 learn.chatgpt.com 对应官方新页。旧 security URL 当前落到另一产品页，未用其证明沙箱行为 |
| Codex Hooks 搜索缓存称默认一秒 | 实际当前 .md 正文多数600秒，Interrupt/SessionEnd另有短预算。采用正文并保留版本验证 |
| Claude 较旧官方博文称默认60秒 | 当前 reference 多数600秒且有 onFailure最低版本，不用旧文覆盖当前合同 |
| 豆包 /work 与 Skill 文档 | 原生HTTP200但可读正文只剩标题，浏览工具也未取得正文。搜索摘要只作线索 |
| Reasonix 官网 v1 与仓库v2 | 保留冲突，生成v2索引不自动证明2.56.0运行时支持 |
| Reasonix Windows沙箱 | 官网与固定CONFIG_PATHS表述冲突，不给已验证隔离结论 |
| Reasonix原生/兼容Hook审批 | Guide原生Hook不能回答审批，插件文档部分兼容描述不同，按不同profile调查 |
| WorkBuddy Enterprise分类的Hook页 | 正文实际CodeBuddy Code，排除为WorkBuddy桌面阻止证据 |
| ZCode配置与设置操作段 | 项目Hook明确忽略，操作段仍提workspace scope，按冲突记录 |
| DSH猜测路径404 | 不存在的 core/tool、tool/shell、subsystems/ptc 路径不作来源，改用真实 tools、pipeline、subprocess 文档 |
| 第三方同名官网/镜像/社区文章 | 只用于发现候选官方入口，不用于认定关键保护能力 |

## 风险

滚动文档可能改变，固定 HEAD 可能超前 release。本次没有完整源码安全审计，未抓取整个站点或存储全文快照。登记中的接口说明为概述，不能替代版本专属契约测试。

## 尚未确定的内容

目标客户端签名、内核与文档版本映射，豆包模式产品身份的具体包构建，缺失文档正文，原生契约矛盾，以及后续测试需要的固定发布包。P1-I01 将其整理为逐版本证据包。


## P0.1新增来源 N01–N18

均为官方/标准正文E1，读取日期2026-11-05，未绑定本机实验。N06与历史S42重复链接是显式复用，不能重复计为独立证据。Windows方案仍全部待实验。

### N01 CreateRestrictedToken

证据E1。受限令牌降低访问，文档建议其它桌面防窗口消息攻击，不能当同SID隔离。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-createrestrictedtoken)

### N02 WFP Access Control

证据E1。OPEN/CLASSIFY与ADD/ADD_LINK权限分开，不能因打开引擎成功宣称可部署过滤。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/fwp/access-control)

### N03 AppContainer isolation

证据E1。token/capability与资源隔离机制，不证明七款实际兼容。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation)

### N04 UWP network troubleshooting

证据E1。WFP/loopback及排障说明，UWP证据不直接外推全部未打包Win32。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/troubleshooting-uwp-firewall)

### N05 Windows Firewall rules

证据E1。显式block优先、应用完整路径与规则条件，真实策略需验证。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/rules)

### N06 Launch an AppContainer

证据E1。与S42同源，启动安全属性及LPAC资源/COM/registry要求。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer)

### N07 CreateProcessAsUserW

证据E1。token访问及配额/assign权限、桌面访问要求按调用条件确定。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessasuserw)

### N08 Desktops

证据E1。桌面是可保护对象，消息/Hook的桌面边界影响GUI自动化。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/winstation/desktops)

### N09 SendMessage

证据E1。UIPI按IL过滤，不能当同IL可信UI。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessage)

### N10 InitializeProcThreadAttributeList

证据E1。扩展创建属性列表的初始化合同。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-initializeprocthreadattributelist)

### N11 UpdateProcThreadAttribute

证据E1。SECURITY_CAPABILITIES/HANDLE_LIST等在创建路径设置，不是运行后追补沙箱。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)

### N12 Process security/access rights

证据E1。进程查询/终止/内存/句柄权限依实际DACL/token，普通同用户不作全面防御。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights)

### N13 CryptProtectData

证据E1。用户/机器数据保护，CurrentUser不构成同SID恶意进程隔离。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)

### N14 BCryptGenRandom

证据E1。系统CSPRNG接口及状态码合同，失败必须拒绝签票。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/api/bcrypt/nf-bcrypt-bcryptgenrandom)

### N15 RFC 4231

证据E1。HMAC-SHA-256算法测试依据，本项目typed framing由18另定义。适用规范来源，不证明真实Agent行为。[官方正文](https://www.rfc-editor.org/rfc/rfc4231)

### N16 UI Automation security overview

证据E1。UIA/UIPI/UIAccess安全前提，不能默认绕过IL和桌面限制。适用规范来源，不证明真实Agent行为。[官方正文](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-securityoverview)

### N17 JSON Schema Validation 2020-12

证据E1。固定Draft及required/条件/边界/format语义，format由验证器显式启用。适用规范来源，不证明真实Agent行为。[官方正文](https://json-schema.org/draft/2020-12/json-schema-validation)

### N18 python-jsonschema validation

证据E1。check_schema、Draft202012Validator与FormatChecker合同，本次使用已有4.26.0。适用规范来源，不证明真实Agent行为。[官方正文](https://python-jsonschema.readthedocs.io/en/stable/validate/)

## 历史58条链接的本轮可达性

通过现有Python urllib发起公开GET，单次12秒、正文采样上限2MiB。2026-11-05共58条，49条HTTP200，1条403，8条超时。S07达到采样上限，S21/S22为产品/前端入口仍不能作A04接口证据，S56虽200但标题含Unknown localization，不据其推导新增技术事实。未读取正文的可达性检查不覆盖历史内容正确性。

| 来源 | 本轮结果 | 使用方式 |
|---|---|---|
| S01 | 超时 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |
| S02 | 超时 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |
| S03 | 超时 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |
| S04 | 超时 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |
| S05 | HTTP403 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |
| S06 | 超时 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |
| S07 | HTTP200 | 正文采样达到2MiB上限，接口仍按原构建边界 |
| S08 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S09 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S10 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S11 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S12 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S13 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S14 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S15 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S16 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S17 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S18 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S19 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S20 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S21 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S22 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S23 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S24 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S25 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S26 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S27 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S28 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S29 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S30 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S31 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S32 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S33 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S34 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S35 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S36 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S37 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S38 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S39 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S40 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S41 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S42 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S43 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S44 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S45 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S46 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S47 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S48 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S49 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S50 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S51 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S52 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S53 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S54 | HTTP200 | 只确认地址响应，能力仍按原证据边界 |
| S55 | TLS握手超时 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |
| S56 | HTTP200 | 标题异常，仅保留历史语言选择依据 |
| S57 | 超时 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |
| S58 | 超时 | 保留P0历史出处，当前可达性未确认，I23复核，不能升证据 |

OpenAI关键页本轮直连不可用，P0.1不据此变更Codex接口或扩大承诺。源码固定提交条目仍不能替代release映射。来源未达或冲突以Q01/Q03/Q05/Q06/Q07/I23跟踪，保留未知。

[能力证据](04-AGENT-CAPABILITY-MATRIX.md) [开放问题](12-OPEN-QUESTIONS.md) [本次验收](P0.1-ACCEPTANCE-REPORT.md)

