# 信任边界与 Windows 11 家庭版安全路线

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05

## 目的

在普通用户权限、无自研驱动、无默认 Windows Service 条件下，界定可实现的控制面、观测面和 OS 隔离能力。

## 结论

第一版采用合作式前置闸门与按能力登记的受限执行路线。Job Object 用于生命周期和资源管理，文件与网络边界必须由令牌、ACL、AppContainer 或经过验证的原生沙箱共同承担。系统级全量监控与不合作 Agent 的透明阻止目前没有可兑现的统一方案。

本机只读 OsName 为 Windows 11 家庭版 中文版，版本 10.0.26200，旧 WindowsProductName 字段仍返回 Windows 10 Home China，因此按 OsName/真实构建判断。未取得补丁修订号或测试 MXC 能力。

## 信任域

| 边界 | 进入的数据/权限 | 信任规则 |
|---|---|---|
| B1 外部内容 → Agent | 文本、脚本、文件、工具返回 | 全部作为不可信数据，不能变更 Guard 策略 |
| B2 Agent/插件 → Adapter | 原始调用、理由、IDs | 声明事件，可能伪造或缺失 |
| B3 Adapter → Core | 规范化提议和证据 | Core 重新验证，拒绝 adapter 自签 allow |
| B4 UI → Core | 人类决策意图、有限参数 | 校验 pending request/digest/期限，UI 不能提交命令执行 |
| B5 Core ↔ Broker / Core → 原生Hook | B用Prepare/Attest/Redeem/Launch，A用deny/defer_to_native | Core独占密钥与ledger，B最终快照不可变，A无兑换时禁连续自动许可 |
| B6 执行环境 → OS 资源 | 文件、注册表、进程、网络 | 依赖实际 token/capability/DACL，非提示词 |
| B7 Core → SQLite/导出 | 脱敏事件、授权状态 | 独占 writer、有限 retention、哈希链仅提高发现率 |
| B8 扩展/解释模块 → Core | 解释、风险建议 | 无授权权，无原始秘密，结果标注推断 |
| B9 更新源 → 本地程序 | binaries、schema、policy | 签名、摘要、最低版本、回滚审计 |

~~~mermaid
flowchart LR
  Agent --> Adapter
  Adapter --> Core
  UI --> Core
  Core --> Ledger
  Core --> NativeHookVerdict
  Broker --> Core
  Broker --> Worker
  Worker --> Resources
  Agent --> UncontrolledPath
  UncontrolledPath --> Resources
  Observer --> Core
~~~

进程分离提供崩溃隔离与最小接口，有效的安全隔离仍取决于 OS 身份、token 和对象权限。同 SID 的两个进程不因位于不同方框而具有强安全边界。

## Windows 能力比较

“无需管理员”仅限调用者有权访问的对象、创建自己的受限进程与自有目录。不自动包含修改系统策略、其他用户资源、任意已运行进程。

| 能力 | 家庭版可用性 | 普通用户可做 | 管理员/特权需求 | 新驱动 | 对 Agent 的影响 | 第一版安排 |
|---|---|---|---|---|---|---|
| 普通用户权限 | 可用 | 运行 Agent/Core | 系统变更仍需提升 | 无 | 通常兼容，但同用户文档和凭证暴露 | 基线，拒绝默认提升 |
| 受限令牌 | Win32 API 可用 | 基于自身 token 降低权限并创建受限子进程 | CreateProcessAsUser 的具体令牌/配额条件需核实，任意用户 token 不适用 | 无 | 安装器、凭证、工具链和 GUI 可能失败 | P1 实验，不能单独声明完整沙箱 |
| Job Object | 可用 | 管理自己创建/可访问的进程组 | 操作外部进程受访问权约束 | 无 | detached server、嵌套 Job、breakaway 有兼容成本 | 受控 launcher 的生命周期管理 |
| AppContainer / LPAC | API 可用，具体应用需验证 | 创建/使用自身 profile 和有权限的 DACL | 修改非自有 ACL 等可能需提升 | 无自研驱动 | 文件、COM、GUI、网络、loopback、工具缓存受限 | P1 隔离 PoC，P1 发布需逐工具验收 |
| ACL | 可用 | 自有且有 WRITE_DAC 的目录/对象 | SACL、系统目录、其他用户权限需额外特权 | 无 | 继承、所有权、工具访问容易冲突 | 仅自有 Guard 数据与专用测试根，禁止默认重写用户盘 ACL |
| Defender AV / CFA | Home 有内置防护，CFA 依赖 active AV | 读取可访问状态、显示建议 | 变更保护策略常需系统权限/用户操作 | OS 自带组件，无新自研驱动 | 可能拦合法文件写入，可信解释器仍可能被滥用 | 与既有防护共存，不自动改策略 |
| ETW | API 可用，权限受 session/provider 限制 | 自有/获授权事件源，部分 in-process trace | 整机 session 通常需 elevated/组授权，Kernel Logger 更严格 | 无新驱动 | 性能、丢事件、敏感命令采集 | 可选诊断，缺权限显示 unavailable |
| Sysmon | 可安装 | 安装后读日志仍受通道 ACL 限制 | 安装 service/driver 需管理员 | 有，微软提供 | 额外维护、磁盘和隐私开销 | 高级可选，第一版不默认安装 |
| Windows Sandbox 产品 | Home 官方不支持 | 无默认路径 | Pro 等版本与启用功能需满足前提 | 非自研，但有系统虚拟化组件 | 需重新安装 Agent，共享目录/网络决定泄露面 | 家庭版基线排除 |
| 虚拟机 | 取决于 hypervisor、硬件和许可 | 已配置 VM 内可操作 | 新装 hypervisor/启用虚拟化常需管理员 | 产品可能带驱动 | 资源高、共享剪贴板/目录/凭证会扩大边界 | 高级隔离方案，具体产品待选 |
| Agent 原生沙箱 | 按版本/平台差异 | 已有且可用时复用 | 例 Codex elevated 初始化需要批准 | 不能笼统推定 | 会影响后台进程、网络、路径 | 作为证据输入，验证后才授予 L3 |

Windows API 依据及各组件职责来自 Microsoft 官方文档。权限例外逐 API 验证，不把进程管理 API 解释为文件/网络监控或逐操作阻止。[S40–50](13-SOURCE-REGISTER.md)

## Codex 新 Windows 路线

当前官方文档描述 mxc、elevated、unelevated 三种实现。MXC 支持取决于系统能力与补丁，原生能力不是 Windows Sandbox 桌面产品。文档提到 CLI 0.162.0+ 的显式检查入口，但本次未运行。Home 不支持 Windows Sandbox 这一事实不能用于推断 MXC 在本机必然可用或不可用。[S03](https://learn.chatgpt.com/docs/windows/windows-sandbox) [S43](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/)

MXC 的 loopback/managed networking、子进程随前台退出停止、deny-read glob 在启动时枚举等限制影响具体保证。P1 必须记录有效实现、补丁和网络策略，不能仅记录一个“sandbox=true”。

## 受控执行路线

设计方案为先建立专用 workspace 与必要工具链只读根，在创建进程时以实际受限token、安全属性和显式继承句柄白名单设置边界，必要时挂起，用户代码执行前完成Job等可延后初始化，再恢复执行。不得在创建后任意附加token/AppContainer并宣称等效。任一设置失败即终止尚未执行的子进程并返回失败。已运行的外部 Agent 不具备可安全追溯加装全部边界的默认承诺。

AppContainer profile 使用专属 SID，最小 capability，broker 暴露结构化资源方法。不得把 Core 的通用读写和任意 shell 能力作为沙箱出口。保留的每个 inherited handle 都必须解释用途，禁止把可写项目外句柄带入。

第一版先验证 OS 边界再增加工具兼容白名单。对于必须访问用户桌面、浏览器登录态的 Agent，当前安全级别以实际可验证范围显示，不能为兼容自动放宽到 full access。

## 系统观察的缺口

Hook 提供语义但可能伪报。进程枚举可错过短命进程。目录 watcher 不提供可靠的全量读取审计，变更也不自动归因到 Agent。ETW/Sysmon 依赖权限、provider、配置和丢失计数。网络元数据不等于文件已上传或对端已接收。

默认无管理员路线不承诺整机每次读文件、网络包、GUI 操作和所有后台进程的完整记录。事件缺失用 unknown 或 gap 表达。

## 技术依据

同用户 token/ACL 的边界限制、AppContainer 的双重访问检查与 Job 的进程管理职责构成选型依据。Sysmon 包含 service/driver，ETW 对跨进程 session 的权限有要求。[S40–48](13-SOURCE-REGISTER.md)

## 风险

AppContainer 对任意现有 Agent 的兼容性尚未证实。受限令牌若仍保留可读凭证、可写敏感句柄或网络能力，不能满足保护资产目标。用户态 Core 被杀后的行为由实际执行器决定。

## 尚未确定的内容

MXC 是否适用本机、Reasonix Windows 隔离冲突、工具链最小访问根、GUI/COM/loopback 兼容性与强制出站控制。上述问题对应 Q03、Q05、Q08、Q10。[开放问题](12-OPEN-QUESTIONS.md)


## P0.1平台约束

A、B、C分别验收。同SID Core/Broker只有故障分离，T2-B防御等待D01。AppContainer、LPAC、Firewall/WFP的网络覆盖和权限全部待实验，网络请求代理不能证明阻止Host直连。受限token桌面建议、UIA/UIPI、COM、剪贴板、OLE、拖放、Shell集成与句柄继承均列独立PoC。

[Broker](15-EXECUTION-BROKER-CONTRACT.md) [网络与权限](17-WINDOWS-NETWORK-ISOLATION.md) [GUI兼容矩阵](19-WINDOWS-ISOLATION-POC.md) [重大决策](P0.1-DECISION-LOG.md)

