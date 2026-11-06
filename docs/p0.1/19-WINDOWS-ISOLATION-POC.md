# Windows Isolation PoC 与 GUI 兼容性

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · AG-07 · 全部实验未执行

## 目的

把受限执行的技术可行性与现有Agent正常功能分别验证，审查桌面、窗口消息、COM、剪贴板和句柄旁路。

## 结论

Restricted Token、AppContainer、LPAC、ACL和Job均只有API/文档层证据。现有七款GUI兼容性全部待验证。不能把“在默认桌面以受限token运行”写成完整GUI安全边界，也不能默认改成隔离桌面而不说明其对自动化的影响。

Microsoft明确建议受限token应用使用其它桌面以降低窗口消息攻击。桌面之间的窗口消息和hook不可直接跨越，UIPI按完整性级别限制消息，但相同IL不是可信UI通道。[N01](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-createrestrictedtoken) [N08](https://learn.microsoft.com/en-us/windows/win32/winstation/desktops) [N09](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessage)

## 兼容性矩阵

| 能力 | 技术可行性 | Agent兼容性 | 权限要求 | 实测状态 |
|---|---|---|---|---|
| Restricted Token | 自身token降权并创建受限进程有公开API | 七款工具链/GUI未验证 | TOKEN_DUPLICATE等，CreateProcessAsUser的quota/assign权限按实际调用核对 | 待验证 |
| AppContainer | 创建时通过SECURITY_CAPABILITIES等属性设置 | Node/Python/Git、浏览器和Host未验证 | profile/所需DACL与创建API权限，非自有系统对象可能需管理员 | 待验证 |
| LPAC | 更严格默认资源访问有文档 | COM、registry和第三方运行库风险较高 | 明确最小capability，不能默认扩大 | 待验证 |
| ACL | 对拥有WRITE_DAC的测试对象限制资源 | 工具cache、临时目录和继承需核对 | 自有对象可普通用户，系统路径不可默认改 | 待验证 |
| Job Object | 生命周期、配额、部分子进程管理 | 嵌套Job、后台server、breakaway和间接激活未验证 | 自有/获权进程句柄 | 待验证，非文件/网络边界 |
| GUI Automation / UIA | 受桌面、IL、UIAccess及应用实现影响 | 七款实际GUI及目标应用未验证 | 跨高IL受限，UIAccess签名/安装/启动条件需专项评估 | 待验证，不默认启用UIAccess |
| UIPI | 按IL过滤部分消息/注入途径 | 同IL常不形成阻隔，低IL驱动高IL可能失效 | 不主动变更系统message filter | 待验证，非同SID通用保护 |
| Window Message | 同桌面可消息交互，跨桌面受限 | 隔离桌面会破坏现有窗口寻址/自动化 | Desktop DACL和线程桌面绑定 | 待验证 |
| COM | 激活、代理、服务与权限模型复杂 | Office/浏览器/Shell自动化可受影响或形成外部执行 | token、COM权限、LPAC能力、现有server身份均需核对 | 待验证 |
| Clipboard | UI/桌面/共享资源影响机密性 | 跨应用复制粘贴可能不可用 | window station/clipboard相关访问权，按实际环境检查 | 待验证，不默认共享敏感剪贴板 |
| OLE | 跨进程数据对象、拖放及COM参与 | 文档生成、嵌入对象和Office路径未验证 | COM/桌面/对象权限 | 待验证 |
| Drag & Drop | 受IL、UIA、OLE、shell数据对象影响 | 跨IL或隔离桌面通常有兼容成本 | 不修改全局安全策略恢复兼容 | 待验证 |
| Shell Integration | Explorer/ShellExecute/文件关联可能委托外部Host | 文件打开/预览可能离开受限进程树 | 验证实际执行者身份，禁止默认使用用户全权限shell出口 | 待验证 |
| 子进程 | 可能继承token/Job，但间接创建不一定属于同一控制树 | MCP、LSP、浏览器、安装器和任务后台路径未验证 | 创建时限制与每个实际process token核验 | 待验证 |
| Handle Inheritance | 显式HANDLE_LIST可限制继承 | stdout、临时文件、socket、pipe依赖须逐项确认 | 句柄最小access mask，跨进程DuplicateHandle需权 | 待验证 |
| 隔离Desktop/Window Station | 有独立可保护UI对象 | Agent和目标软件需在可交互范围配对，现有桌面自动化可能失效 | 自建对象、DACL、thread desktop限制，部分操作权限要求待测 | 待验证 |
| Network IPC/loopback | 可作为工具/浏览器依赖，也可能绕过网络边界 | 仅代理方案与现有server共存未知 | 不默认添加loopback exception或Firewall规则 | 待验证，另见网络计划 |

AppContainer/LPAC文档说明默认访问、COM和registry能力，创建属性文档说明安全能力、Job及继承句柄的创建时参数。文档证据不能证明七款的二进制兼容。[S42、N07、N10、N11](13-SOURCE-REGISTER.md)

## 七款功能回归登记

| Agent | 无损功能基线 | 需要验证的隔离路径 | 当前状态 |
|---|---|---|---|
| Codex | 本地工具、终端持续输入、MCP、文件变更 | native sandbox实现与Broker recipe分别 | 未验证 |
| Claude Code | Shell、读写、MCP、子Agent、插件Hook | 进程/桌面/COM等与原生权限协作 | 未验证 |
| DSH | Electron Host、Cordis插件、PTC、LSP/PTY | Host整体或仅受控Worker，plugin直接OS行为 | 未验证 |
| 豆包 App 内工作模式 | 用户已确认App内模式，Windows构建待确认 | GUI/本地/云路径，公开委托点未知 | 未验证，具体包与构建未知 |
| Reasonix | 原生Hook、Extension v2、工具与子Agent | Windows实际sandbox实现及full-trust扩展 | 未验证，冲突保留 |
| WorkBuddy | 本地/云任务、原生安全中心、插件和审计 | Desktop工具、GUI和后台子任务 | 未验证 |
| ZCode | 用户级Hook、MCP、子Agent、文件/终端 | 新session配置和真实process边界 | 未验证 |

## 实验阶段及准入

I07-A为提前隔离可行性实验，仅使用自行审查的合成worker和虚拟文件树，依赖I02-B环境与I20-A供应链基线，不等待七款Adapter完成。不得先给真实Agent安装目录加ACL或在真实用户目录做测试。

I07-B在I06最终兑换合同、I22生命周期和I07-A可行性通过后，验证Broker/Worker的文件、网络、GUI/COM/句柄边界。I10–I16按各自版本条件开展真实兼容性测试，I17使用结果做独立认证。

| PoC ID | 内容 | 双重验收 |
|---|---|---|
| ISO01 | 创建受限token/AppContainer/LPAC、捕获SID/LUID/IL/token digest | 创建失败必须安全结束，实际身份匹配 |
| ISO02 | Job、继承handle白名单、挂起启动与创建时security属性 | 用户代码前初始化成功，越权句柄不可得 |
| ISO03 | 虚拟树读写删、reparse/hardlink/替换目录 | 合法任务成功，越界实际失败 |
| ISO04 | 默认桌面/独立桌面、低/中IL消息与UIA | 合法GUI任务可完成，拒绝的消息/控制路径有独立证据 |
| ISO05 | COM/OLE/Clipboard/拖放/ShellExecute | 正常功能及外部代理身份透明，无沙箱出口越权 |
| ISO06 | 子进程/间接激活/断开/崩溃/Job句柄遗留 | 生命周期真实可查，无法停的外部执行明确列范围外 |
| ISO07 | IPv4/6、TCP/UDP/QUIC、loopback、已建立连接 | 合法网络recipe成功，未授权出口无sink接收 |
| ISO08 | Core/Broker/Worker故障时点及恢复 | 无旧nonce再启，未知结果不重试 |

每项记录OS build+修订号、账户/权限、签名binary摘要、实际DACL/token/desktop/Job、功能baseline、允许与拒绝对照、independent observer和sink结果。合法功能通过而安全失败，或安全通过而关键功能损坏，都不算可发布组合。

## 兼容失败处理

若隔离破坏某Agent关键功能，保留A模式的已验有限拒绝或L1观察，明确C未取得认证。不得在同一B授权下自动切回默认桌面、全权限token或用户浏览器登录态。需要共享剪贴板/放宽COM/UIAccess的方案提交D03风险决定，另建范围并重测。

## 技术依据

Windows公开文档支持机制存在及其边界，P0.1没有调用这些API建立沙箱。[新增Windows来源](13-SOURCE-REGISTER.md) [网络实验](17-WINDOWS-NETWORK-ISOLATION.md)

## 风险

将受限Agent接到默认桌面和全权限COM服务可能重新引入不受控操作。把七款全部迁入隔离桌面又可能失去它们的GUI正常功能。此取舍需要产品范围与证据共同决定。

## 尚未确定的内容

D03、实际管理员需求、GUI正常功能集合、每款可用的受限组合、创建API权限例外及实际网络强制结果。全部实验未执行。

