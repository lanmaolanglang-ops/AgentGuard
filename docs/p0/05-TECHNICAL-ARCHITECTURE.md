# 总体技术架构

AgentGuard V2.0 · P0-5 · 2026-11-05

## 目的

确定 UI、适配器、策略、授权、执行边界及本地审计的职责和故障隔离。以下组件和数字均为设计提案，尚无实现或性能实测。

## 结论

采用 Tauri 2 + React + TypeScript 桌面 UI，独立 Rust 用户态 Core，独立 Adapter 进程，Windows Named Pipes 与本地 SQLite。确定性策略和授权账本放在同一 Core 事务域。第一版保持无默认 Windows Service、无自研驱动、无默认管理员初始化。

Core 是用户会话内后台进程，生命周期独立于 UI。Tauri Core/sidecar 本身的父子关系不能替代独立退出语义，关闭 UI 与退出防护为两个显式操作。Core 崩溃后的安全由实际强制入口负责，UI 重启不是安全恢复证明。

## 架构图

~~~mermaid
flowchart TB
  subgraph PRODUCTS["七款产品，按构建单独适配"]
    A["Codex / Claude Code / DSH / 豆包工作模式"]
    B["Reasonix / WorkBuddy / ZCode"]
  end
  A --> AD["Adapter 进程集合，观察和请求"]
  B --> AD
  AD --> PIPE["有身份与限额的 Named Pipes"]
  PIPE --> CORE["Rust Core"]
  CORE --> NORMAL["严格解析与证据分类"]
  NORMAL --> POLICY["确定性规则与授权账本"]
  POLICY --> CONTROL["同步决策通道"]
  CONTROL --> AD
  POLICY --> DB["SQLite 单 writer，事件与授权事务"]
  DB --> VIEW["只读投影视图"]
  VIEW --> UI["Tauri 2 / React / TypeScript"]
  UI --> INTENT["有限审批意图"]
  INTENT --> POLICY
  NORMAL --> EXPLAIN["本地模板解释，独立可选解释 worker"]
  EXPLAIN --> UI
  AD --> HOST["产品原生同步执行点"]
  HOST --> BOUNDARY["已验证原生沙箱 / 受控隔离进程"]
  BOUNDARY --> RESULT["执行结果与独立观察"]
  RESULT --> AD
~~~

## 组件合同

| 组件 | 输入与输出 | 权限及限制 |
|---|---|---|
| UI | 脱敏快照、待审批条目 → 用户审批意图 | 无任意 shell、无数据库写权限、无 policy 文件编辑 |
| Core | 提议/证据/意图 → policy decision、许可、审计 | 仅自有数据目录及必要 OS 查询，无通用执行 RPC |
| Policy module | 规范化不可变操作 → deny/ask/allow/native_fallback | Rust 纯确定性模块，和 ledger 共用事务边界 |
| Adapter | 版本专属 payload ↔ 通用 ingress/verdict | 验格式、保留来源，不能自定最终 risk 或 permit |
| Hook shim | stdin → Core IPC → 产品规范 verdict | 短生命周期，固定二进制、最小依赖，异常转换按产品合同 |
| Runtime plugin | DSH/Reasonix seam ↔ out-of-process bridge | 必须承认宿主内部分信任，外置 bridge 不限制插件直接 OS 权限 |
| Audit writer | 事件/授权原子记录 → durable ack | Core 唯一写者，禁止 Adapter 直接写表 |
| Explanation | 已脱敏结构化事实 → 中文模板/标注推断 | 无决策权，无新权限，无默认远端 LLM |
| Execution boundary | 原生工具或专用 broker 的 operation envelope | 实际行为必须匹配许可，非合作 host 无法保证 |
| Optional sensors | 可获得的进程/文件/网络元数据 | 丢失计数和权限状态必备，不能自动改变 policy |

Tauri capability 约束 UI IPC，Electron 也有成熟隔离配置，两者都不能自动控制外部 Agent。采用 Tauri 的理由是 UI 需求有限且 Rust Core 已确定，减少 Node 主进程桥接面。成本是 WebView2 漂移与 Windows 集成测试。[S51–53](13-SOURCE-REGISTER.md)

## 主数据流

1. Adapter 建立受认证连接，声明 agent/runtime/adapter/tool/provider/config/generation 摘要。Core 将其与验收 profile 比较。
2. 接收 proposed 事件，记录 ingress ID 和接收时间。Core 严格解析，收集可得的独立身份、路径和内容证据。
3. Core 生成 operation_digest，先匹配硬禁，再验证有限授权。缺失或无法判断进入 ask/deny。
4. 需要人工时写入 pending request，UI 显示此 digest 对应的快照。原生 Hook 预算不足时先拒绝并引导重新提议。
5. UI 返回 pending_id/digest/decision/scope。Core 原子核对、计数、撤销 epoch，持久记录后发响应。
6. 最终执行点兑换一次性许可并校验最终参数。只有具备该控制点的集成可承诺参数与次数保证。
7. 产品结果和 OS 证据分别入库，投影形成回放时间线。缺少结果不补造成功。

## 本地 IPC

采用长度前缀 UTF-8 JSON frame，首部为协议 major/minor 与消息类型，默认每帧上限 1 MiB，受控大 payload 使用单独有界内容引用。数值为初始设计预算，P1 验证后冻结。

连接使用显式 DACL、当前 logon session/user 身份范围、拒绝远程客户端、first-instance 防抢占、实际 client/server PID 与创建时间核对。角色分为 UI、Adapter、sensor，分别只开放必要方法。不要长时间 impersonate 客户端，避免 confused deputy。

每个会话握手 nonce 与短期连接密钥抵御意外串线和跨连接重放。启动实例认证、二进制摘要和签名增信仍不足以抵抗同 SID 内存读取/注入，需要在信任模型保留此限制。pipe 名和可执行文件名均不是身份凭证。默认 pipe 权限不能直接采用。[S45](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)

本地 TCP 作为未来跨平台可选方案需要 token、Origin/CSRF、防端口抢占及禁止外网监听。第一版 Windows 本地场景选择 Named Pipes，减少浏览器可访问网络端点。

## 队列、背压与持久化

控制请求使用独立有界高优先队列，Adapter 并发和每会话 pending 数设配额。观察事件使用有界队列，饱和时丢弃低优先样本并记录精确计数/区间，不能丢弃允许前的授权记录。

SQLite 使用单 writer task，读操作用只读连接和短事务。WAL 可改善读写并发，同一时间仍只有一个 writer。策略授权与 decision event 同一数据库事务提交，设置满足 durability 的同步策略，busy/满盘/IO 错误时拒绝新许可。WAL 不放网络共享目录。[S54](https://sqlite.org/wal.html) [S55](https://sqlite.org/whentouse.html)

初始设计为控制队列 256、观察队列 10000、每 Adapter 16 个并发请求、单次 IPC 2 秒软预算。它们是待测配置，不作为产品指标。数据库 checkpoint、日志保留、导出和备份不得长期持有写事务。长期原始大 blob 默认不存。

## 安全审计与回放

事件 append-only，接收顺序 ingest_seq 与源序列分开。原始敏感字段在 Adapter 采集边界先脱敏，Core 再做防御性脱敏。决策只在内存中使用完整参数并以域分离 HMAC 绑定，脱敏文本不能用于授权匹配。

链记录以确定性序列化后的脱敏 envelope、prev_hash、ingest_seq 计算摘要。日志段封口形成 checkpoint，用户可显式导出到独立介质保存锚点。没有独立锚点时无法检测对手整体重算或尾部截断。本地 DPAPI/ACL 可以减少意外泄露，不能保证抗同用户恶意进程。已脱敏事实和解释版本同时保存，回放不重新运行命令、不自动联网解释、不自动复用历史批准。

文件变化按 reported/observed/diff_verified 分级。没有授权采集内容时只存元数据，备份保留策略与审计保留分开。

## 故障处理

| 故障 | Core 的设计行为 | 真实强制条件 |
|---|---|---|
| UI 关闭/崩溃 | 已授权且有效的受控操作可继续，新的人工审批 pending，期限到即拒绝 | Core 独立运行，UI 不持有关键 Job 最后句柄 |
| Core 崩溃 | 新许可停发，重启 epoch 失效旧许可，进行未决操作对账 | host timeout 必须拒绝，或执行器随 Core 所有 Job 句柄关闭而终止 |
| Adapter 崩溃/断线 | 降级、停止新许可、保留未知结果 | host 是否继续取决于其实现，不能统一宣称已停 |
| DB 满/损坏/busy 超预算 | 禁止新 allow，保留最小错误提示，恢复后追加 gap | 已发出许可无法靠落盘失败自动收回 |
| sensor 丢事件 | 标记 evidence gap，相关保护声明失效 | 仅语义 pre 仍可工作，所缺证据必须显示 |
| Agent 更新 | profile 失配，撤销所有可复用授权并降至观察/未接入 | 不能在更新期间保持原绿色标识 |
| Core 正常退出 | 撤销、等待/停止受控执行、记录未决条目 | 对外部 host 只记录已证实的停止结果 |

“回退原生审批”只在产品保证真正等待且不会自动许可时可启用。Core 不可达、未知操作、命中硬禁时不能静默转为 host allow。无可靠原生回退的集成拒绝或降级，详见策略引擎。

## 技术依据

前置闸门失效语义决定独立执行边界必要性，SQLite 单 writer 决定本地 writer 架构，Tauri 模型决定 UI capability 设计。[S01、S07、S13、S24–26、S45、S51–55](13-SOURCE-REGISTER.md)

## 风险

多进程有协议和部署成本，用户态进程仍共享同 SID 风险。独立 Core 无法修复第三方 host 的 fail-open，哈希链也无法单独提供不可篡改审计。

## 尚未确定的内容

Core 启动/重启方案、IPC 句柄认证实测、数据库版本与耐久策略、AppContainer/broker 的最小工具契约，以及平台整机观察缺口。[ADR](11-TECHNOLOGY-DECISIONS.md)

