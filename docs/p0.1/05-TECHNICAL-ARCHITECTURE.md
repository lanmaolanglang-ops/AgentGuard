# 总体技术架构

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05

## 目的

确定 UI、适配器、策略、授权、执行边界及本地审计的职责和故障隔离。以下组件和数字均为设计提案，尚无实现或性能实测。

## 结论

采用 Tauri 2 + React + TypeScript 桌面 UI，独立 Rust 用户态 Core，独立 Adapter 进程，Windows Named Pipes 与本地 SQLite。确定性策略和授权账本放在同一 Core 事务域。第一版保持无默认 Windows Service、无自研驱动、无默认管理员初始化。

Core 是用户会话内后台进程，生命周期独立于 UI。Tauri Core/sidecar 本身的父子关系不能替代独立退出语义，关闭 UI 与退出防护为两个显式操作。Core 崩溃后的安全由实际强制入口负责，UI 重启不是安全恢复证明。

## 架构图

~~~mermaid
flowchart TD
  Agent --> Adapter
  Adapter --> IPC
  IPC --> Core
  Core --> Policy
  Policy --> Ledger
  Ledger --> Projection
  Projection --> UI
  UI --> UserIntent
  UserIntent --> Core
  Core --> NativeVerdict
  Adapter --> Broker
  Broker --> Core
  Broker --> Worker
  Worker --> Resources
  Worker --> Result
  Observer --> Evidence
  Result --> Core
  Evidence --> Core
  Core --> Explanation
  Explanation --> UI
~~~

NativeVerdict为A模式，Broker/Worker为B，Worker的OS边界为C。Result仅执行器报告，Evidence独立验证。Core是唯一ledger与密钥持有者。所有组件均为待实现设计。

## 组件合同

| 组件 | 输入与输出 | 权限及限制 |
|---|---|---|
| UI | 脱敏快照、待审批条目 → 用户审批意图 | 无任意 shell、无数据库写权限、无 policy 文件编辑 |
| Core | 提议/证据/意图 → policy decision、许可、审计 | 仅自有数据目录及必要 OS 查询，无通用执行 RPC |
| Policy module | 规范化不可变操作 → deny/ask/allow/defer_to_native | Rust纯确定性模块，和ledger共用事务边界 |
| Adapter | 版本专属 payload ↔ 通用 ingress/verdict | 验格式、保留来源，不能自定最终 risk 或 permit |
| Hook shim | stdin → Core IPC → 产品规范 verdict | 短生命周期，固定二进制、最小依赖，异常转换按产品合同 |
| Runtime plugin | DSH/Reasonix seam ↔ out-of-process bridge | 必须承认宿主内部分信任，外置 bridge 不限制插件直接 OS 权限 |
| Audit writer | 事件/授权原子记录 → durable ack | Core 唯一写者，禁止 Adapter 直接写表 |
| Explanation | 已脱敏结构化事实 → 中文模板/标注推断 | 无决策权，无新权限，无默认远端 LLM |
| Execution Broker | Prepare、AttestPrepared、Redeem、Launch、Cancel、Reconcile | 独立进程，无Core密钥，typed recipe白名单，禁止通用提权shell RPC |
| Worker / Network Broker | 受限身份实际操作与typed网络请求 | 仅持有最小句柄，网络代理需独立C边界才证明无直连 |
| Independent Observer | sentinel、文件identity/摘要、进程实例、sink receipt | 独立证据仍须核实权限/可见范围，缺失保留unknown |
| Optional sensors | 可获得的进程/文件/网络元数据 | 丢失计数和权限状态必备，不能自动改变 policy |

Tauri capability 约束 UI IPC，Electron 也有成熟隔离配置，两者都不能自动控制外部 Agent。采用 Tauri 的理由是 UI 需求有限且 Rust Core 已确定，减少 Node 主进程桥接面。成本是 WebView2 漂移与 Windows 集成测试。[S51–53](13-SOURCE-REGISTER.md)

## 主数据流

1. Adapter提交声明，Core核对实际peer、精确profile和协议major，审计副本与权威操作数据分流。
2. A仅返回deny/ask/defer_to_native，原生执行仍归Agent。无最终点时不能发强绑定单次许可。
3. B在明确原生allow下先创建pending grant，Broker按有界准备许可固定最小资源与最终Worker身份。
4. Broker提交最终OperationSnapshot，Core按18计算semantic供UI确认，确认后在预占事务生成nonce/reservation、operation摘要和绑定prepared_id的一次性permit。
5. Broker在线Redeem，Core在同一durable事务消费nonce和记录审计，之后Broker只启动已持有的不可变操作。
6. Worker结果和独立观察分别记录。consume响应丢失/崩溃不自动重放，已消费历史保持consumed。
7. C以独立证书约束实际资源，无OS证据的Host直连和GUI/COM路径保持覆盖外。

[完整执行合同](15-EXECUTION-BROKER-CONTRACT.md) [数据结构](07-UNIFIED-EVENT-SCHEMA.md)

## 本地 IPC

采用长度前缀 UTF-8 JSON frame，首部为协议 major/minor 与消息类型，默认每帧上限 1 MiB，受控大 payload 使用单独有界内容引用。数值为初始设计预算，P1 验证后冻结。

连接使用显式 DACL、当前 logon session/user 身份范围、拒绝远程客户端、first-instance 防抢占、实际 client/server PID 与创建时间核对。角色分为 UI、Adapter、Broker、sensor，分别只开放必要方法。不要长时间 impersonate 客户端，避免 confused deputy。

握手challenge与请求序列用于抵御意外串线及重放。首版依赖OS peer认证和受保护pipe，不向Adapter分发HMAC许可密钥。若新增连接密钥必须独立设计，不得复用Core许可密钥。启动实例认证、二进制摘要和签名增信仍不足以抵抗同 SID 内存读取/注入，需要在信任模型保留此限制。pipe 名和可执行文件名均不是身份凭证。默认 pipe 权限不能直接采用。[S45](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)

本地 TCP 作为未来跨平台可选方案需要 token、Origin/CSRF、防端口抢占及禁止外网监听。第一版 Windows 本地场景选择 Named Pipes，减少浏览器可访问网络端点。

## 队列、背压与持久化

控制请求使用独立有界高优先队列，Adapter 并发和每会话 pending 数设配额。观察事件使用有界队列，饱和时丢弃低优先样本并记录精确计数/区间，不能丢弃允许前的授权记录。

SQLite 使用单 writer task，读操作用只读连接和短事务。WAL 可改善读写并发，同一时间仍只有一个 writer。策略授权与 decision event 同一数据库事务提交，设置满足 durability 的同步策略，busy/满盘/IO 错误时拒绝新许可。WAL 不放网络共享目录。[S54](https://sqlite.org/wal.html) [S55](https://sqlite.org/whentouse.html)

初始设计为控制队列 256、观察队列 10000、每 Adapter 16 个并发请求、单次 IPC 2 秒软预算。它们是待测配置，不作为产品指标。数据库 checkpoint、日志保留、导出和备份不得长期持有写事务。长期原始大 blob 默认不存。

## 安全审计与回放

事件 append-only，接收顺序 ingest_seq 与源序列分开。Adapter在进入日志前脱敏，另经受保护控制通道提交完整权威操作给Core/Broker，后者在内存与prepared资源中核对。Core在持久化前再次脱敏，完整执行数据不能被采集脱敏永久丢失，脱敏文本也不能用于授权匹配。

审计按18的audit-v2域、segment_id/sequence/previous_mac/event_digest/key_id封链。日志段封口形成 checkpoint，用户可显式导出到独立介质保存锚点。没有独立锚点时无法检测对手整体重算或尾部截断。本地 DPAPI/ACL 可以减少意外泄露，不能保证抗同用户恶意进程。已脱敏事实和解释版本同时保存，回放不重新运行命令、不自动联网解释、不自动复用历史批准。

文件变化按 reported/observed/diff_verified 分级。没有授权采集内容时只存元数据，备份保留策略与审计保留分开。

## 故障处理

| 故障 | Core 的设计行为 | 真实强制条件 |
|---|---|---|
| UI 关闭/崩溃 | 已授权且有效的受控操作可继续，新的人工审批 pending，期限到即拒绝 | Core 独立运行，UI 不持有关键 Job 最后句柄 |
| Core 崩溃 | 新许可停发，重启 epoch 失效旧许可，进行未决操作对账 | B未消费不能启动，已消费对账与取消需实测，A的host timeout行为单独披露 |
| Adapter 崩溃/断线 | 降级、停止新许可、保留未知结果 | host 是否继续取决于其实现，不能统一宣称已停 |
| DB 满/损坏/busy 超预算 | 禁止新 allow，保留最小错误提示，恢复后追加 gap | 已发出许可无法靠落盘失败自动收回 |
| sensor 丢事件 | 标记 evidence gap，相关保护声明失效 | 仅语义 pre 仍可工作，所缺证据必须显示 |
| Agent 更新 | profile 失配，撤销所有可复用授权并使依赖变化的证书失效，按剩余独立能力重新计算范围等级 | 不能在更新期间保持原绿色标识 |
| Core 正常退出 | 撤销、等待/停止受控执行、记录未决条目 | 对外部 host 只记录已证实的停止结果 |

“回退原生审批”只在产品保证真正等待且不会自动许可时可启用。Core 不可达、未知操作、命中硬禁时不能静默转为 host allow。无可靠原生回退的集成拒绝或降级，详见策略引擎。

## 技术依据

前置闸门失效语义决定独立执行边界必要性，SQLite 单 writer 决定本地 writer 架构，Tauri 模型决定 UI capability 设计。[S01、S07、S13、S24–26、S45、S51–55](13-SOURCE-REGISTER.md)

## 风险

多进程有协议和部署成本，用户态进程仍共享同 SID 风险。独立 Core 无法修复第三方 host 的 fail-open，哈希链也无法单独提供不可篡改审计。

## 尚未确定的内容

Core 启动/重启方案、IPC 句柄认证实测、数据库版本与耐久策略、AppContainer/broker 的最小工具契约，以及平台整机观察缺口。[ADR](11-TECHNOLOGY-DECISIONS.md)


## 组件到任务映射

Core/IPC/Schema为I03，Policy为I04，ledger为I05，Broker/Worker/Network Broker合同为I06，OS强制点为I07-A/I07-B，审计/解释为I08，UI为I09，Adapter/shim/plugin为I10–I16，Observer/sensor为I21，生命周期为I22，安装为I18，供应链为I20-A/I20-C。I17独立认证，I24最终发布。[精确依赖图表](14-P1-DEVELOPMENT-PLAN.md)

