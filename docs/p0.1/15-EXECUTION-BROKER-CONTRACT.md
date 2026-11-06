# Execution Broker 执行控制合同

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · AG-01 / AG-06 · 规范性设计

## 目的

补全“策略批准”到“最终操作”的控制链，定义进程、IPC、参数、身份、nonce、取消与恢复。本文是拟实现合同，当前无Broker程序或运行证明。

## 结论

Execution Broker是独立于UI、Adapter和Core的受信执行进程，只接受已登记的typed recipe。Core保留唯一授权账本和所有HMAC密钥，Broker通过Core的Redeem接口兑换。Worker执行实际操作，持有最小资源句柄和所需受限身份。独立进程提供故障分离，同SID部署仍不能抗T2-B。

三种模式分别认证。七款是否能把某操作委托给Broker全部待验证，现有Hook接口证据不能作为委托证据。

## 三种模式

| 模式 | 实际执行者 | Core输出 | 可承诺范围 | 失效处理 |
|---|---|---|---|---|
| A 原生前置阻止 | Agent原生工具 | native_deny或defer_to_native | 已验入口显式拒绝，无最终点时无强参数绑定/单次保证 | 记录host实际故障行为，禁连续自动许可 |
| B 委托执行 | Broker/受限Worker | typed reservation与一次性permit | 仅经最终核对及消费的recipe，所需资源边界另验 | 未消费不启动，消费后故障按unknown/取消处理，无隐式原生回退 |
| C OS独立限制 | 被实际纳入边界的Host/Worker | 隔离策略状态，单独认证 | 声明的文件/网络/进程/GUI资源 | OS策略失效时撤销依赖许可，A可独立保留 |

A不要求OS隔离。B也不自动获得OS隔离，未具备该操作需要的边界则不启动相应自动操作。C不证明所有Hook正常或Agent最终参数一致。[统一能力定义](16-PROTECTION-CAPABILITY-CONTRACT.md)

## 进程和权限

| 组件 | 权限、状态与接口 |
|---|---|
| UI | 提交用户确认意图，只读脱敏视图，不能创建Worker |
| Adapter / Host桥 | 提议操作，不直接向Worker传参数或持有Core密钥 |
| Core | 决策、grant账本、epoch、MAC核验、capability认证和审计唯一writer |
| Execution Broker | 验typed recipe、捕获OS身份与资源、Prepare/Redeem/Launch/Cancel/Reconcile |
| Worker | 实际文件、网络或子进程recipe，仅持有执行必需句柄，禁止访问Core账本/密钥 |
| Independent Observer | 与Worker结果分开提供可得的进程、文件、受控sink证据，独立身份仍须验证 |

Broker初期与Core可使用同一用户普通权限，设计安全目标限定T0/T1/T2-A。若选择独立安全主体/服务抵御T2-B，需项目负责人决定部署与权限成本，见D01，当前未批准。

~~~mermaid
flowchart LR
  Agent --> Adapter
  Adapter --> Core
  UI --> Core
  Core --> Ledger
  Adapter --> Broker
  Broker --> Core
  Broker --> Worker
  Worker --> Resource
  Observer --> Core
~~~

## 操作快照与身份绑定

OperationSnapshot的结构以07中的operation-v2为准，包含agent/runtime/adapter/provider/config、session/invocation、requestor_principal、executor_principal、recipe、command、typed arguments、cwd身份、target对象身份、executable/script/input内容摘要、environment_digest、native_permission和network_authorization。

Broker核验请求连接peer的PID、进程创建时间、SID、logon LUID、IL、token限制及binary摘要，分别记录Agent自报与OS核验值。请求发起者与实际Worker身份不同是正常委托，必须同时绑定两者。仅核对Adapter的PID不足以证明AgentHost身份，Host关联缺证据时禁止强绑定许可。

executor_principal在Prepare时捕获，其sandbox_identity.policy_digest与计划执行边界匹配。user SID或runtime/provider/generation变化即新操作。进程路径、签名和PID均不能替代进程实例及当前token核验。

## IPC方法合同

方法使用有界Named Pipe消息，显式DACL、远程拒绝、连接角色和Peer OS校验。所有ID为数据，禁止把客户端传入整数解释为本地HANDLE。跨进程传句柄必须由Broker在核对源进程和允许对象类型后DuplicateHandle，随后验证对象真实身份与访问掩码，拒绝进程/token/未知对象句柄及多余写权限。可行性由AT44验证。

| 方法 | 方向 | 输入与必需验证 | 返回/效果 |
|---|---|---|---|
| Prepare | Core登记pending后，Adapter→Broker | request_id、grant_id、完整typed操作及Core准备许可，校验recipe/native限制/期限 | prepared_id、Broker实例、快照摘要、受控对象identity，无业务写入/上传 |
| AttestPrepared | Broker→Core | prepared_id、原始安全快照的确定编码、OS核验identity、对象清单 | Core重算semantic_digest，冻结供UI确认的最终快照，确认后预占并签发permit |
| Redeem | Broker→Core | permit、prepared_id、当前epoch与最终快照 | 原子reserved→consumed，成功一次返回launch_authorization |
| Launch | Broker内部 | 持有同一prepared快照和一次授权，最终安全状态再核对 | 进入recipe，不重新读取Host参数 |
| Cancel | Core→Broker | execution_id、generation、取消原因、截止时间 | 取消请求ack，终止证据另报，不能把ack当停止成功 |
| ReportResult | Broker→Core | worker结果、exit状态、变化摘要 | worker_reported结果，独立证据另记录 |
| Reconcile | Broker↔Core | boot/generation、未决prepared/execution集合 | 保留unknown，终止未决Worker或按已批准恢复合同处理 |
| Heartbeat | 双向 | instance、last_seq、lease_deadline | 只证明连接健康，不能替代OS强制状态认证 |

方法参数用07中的正式结构，缺失安全字段、重复JSON键、未知recipe/major、畸形MAC、超预算帧全部拒绝。核心字段不允许从extensions补值。

OperationSnapshot.handle_manifest由Broker实际查询后提交，逐项包含opaque handle_ref、object_type、object_identity_digest、granted_access、inherit_to_worker和purpose。handle_ref只是prepared内部映射UUID，不能作为本地HANDLE解引用。对象identity摘要按SHA-256(E(实际对象身份))计算，仅由Broker/Core生成，不接受客户端自填。完整manifest参与semantic_digest，最终Launch核对同一对象与access，拒绝附加/替换/更宽句柄和未登记继承资源。AT44验证这一合同。

## 准备、消费与启动顺序

1. Core收到提议生成created→pending grant。硬禁、原生deny或强制点不足时拒绝。
2. Core仅登记有界准备许可，grant仍为pending。它只允许Broker准备已审查recipe的必要资源，不是reserved ticket或业务执行许可，准备配额与短deadline独立管理。
3. Broker固定typed操作、以只读/最小权限打开候选资源，建立专用临时状态。可能创建的空临时对象/挂起进程作为prepare副作用单独记录，不宣称绝对零系统副作用。准备不得读取并外发文件内容、运行用户代码或提交业务写入。
4. 对process recipe，token、AppContainer安全能力、句柄白名单、桌面及Job的适用设置必须在创建时或用户代码运行前完成。token/AppContainer不能在任意进程创建后追补为可靠隔离。初始化失败终止挂起Worker，不退回无隔离执行。
5. Broker向Core提交最终快照，Core计算semantic_digest，UI批准这一冻结快照。之后Core在预占事务生成reservation/nonce，计算最终operation_digest，记录reserved和permit。最终身份在准备后才能确定，因此禁止先发行含占位digest的reserved ticket。安全相关对象已变、native policy未知或许可过期则拒绝并重新提议。
6. Core在一个durable事务中核对grant状态/epoch/expiry/peer/prepared_id、唯一nonce及最终摘要，更新reserved→consumed并写同一审计event。这是授权线性化点。
7. Core返回绑定prepared_id/worker_instance/Broker实例的launch_authorization。Broker只启动其已经持有的同一快照，一次处理launch响应。重复、迟到或失联重试响应仅可查询状态。
8. 启动后分别记录execution.started、worker结果、独立副作用证据。缺少started不能补造，未知结果不能自动重跑。

消费成功到实际OS副作用之间没有跨进程/OS/SQLite的分布式原子事务。设计只保证最多一次授权消费和受控启动尝试，系统崩溃可能损失一次额度，无法承诺任意外部副作用exactly-once。

## Grant与执行状态分离

Grant只有created、pending、reserved、consumed、revoked、expired、invalidated、failed。consumed是最终授权状态。工具失败不会把consumed改为failed，后者仅表示尚未消费的授权合同失败。

Execution独立使用prepared、launch_authorized、running、succeeded、failed、cancelled、unknown。准备失败可使未消费grant进入failed。消费后启动失败只写execution.failed，额度不返还。

~~~mermaid
flowchart TD
  Created --> Pending
  Pending --> Reserved
  Reserved --> Consumed
  Created --> Revoked
  Pending --> Revoked
  Reserved --> Revoked
  Created --> Expired
  Pending --> Expired
  Reserved --> Expired
  Created --> Invalidated
  Pending --> Invalidated
  Reserved --> Invalidated
  Created --> Failed
  Pending --> Failed
  Reserved --> Failed
~~~

图中Created等标签对应协议小写枚举。终态之间转换非法，重复查询不生成第二次消费。合法转换的触发条件、审计一致性见08，AT45覆盖全部合法/非法转换。

## 文件、网络与子进程边界

文件recipe使用经验证的根、对象identity和内容摘要。Windows路径解析、reparse/hard-link、替换目录与跨卷问题按独立PoC验证，无法可靠锁定则拒绝。Broker不得暴露任意路径读写或把自己的全权限句柄交给Worker。

网络recipe由受控Network Broker执行精确HTTP(S)请求，验证DNS/IP、每跳重定向、方法和正文摘要，限制每次字节数/请求数。它只约束经过其接口的请求。Worker/Host直连必须由另行验证的OS机制阻止，否则只称请求代理。[网络合同](17-WINDOWS-NETWORK-ISOLATION.md)

process recipe绑定可执行体、确切Windows命令行字符串和各工具专属argv序列化器。CreateProcessW的命令行不保证所有程序遵循同一argv解析规则，不能把数组参数自然等同于实际argv。首版推荐先实现结构化文件recipe，任意Shell和持续stdin为待决范围D04。

Job负责进程树管理。继承句柄、breakaway、COM/Shell间接激活、现有外部进程和MCP远端执行均须逐项验收，不能从“在Job里”推导全量资源限制。

## 取消、撤销与崩溃恢复

| 时点 | 设计行为 | 保证边界 |
|---|---|---|
| 撤销事务先于消费提交 | reserved→revoked，拒绝Redeem，清理prepare状态 | 禁止新的该ticket启动授权 |
| 消费先于撤销 | grant保留consumed，关联cancel请求及撤销意图 | 可能已经产生副作用，按取消结果显示 |
| Core不可达且未消费 | Broker禁止Launch，期限到终止prepare | 无离线permit兑换 |
| 消费响应丢失 | 查询返回consumed状态，不返回可重复启动授权，Worker终止/unknown | 不自动退额度或重启 |
| Core重启 | 新core_boot_epoch，旧未消费ticket invalidated，审计对账 | 已consumed仍是历史事实 |
| Broker崩溃 | 依赖持有方式经验证的Job/Worker lease，独立对账 | 不承诺所有Job句柄必关闭或全部外部子进程已结束 |
| Worker timeout | 尝试终止受控执行树，记录实际exit和sink证据 | 已发送网络数据和已写内容不能追回 |
| 审计持久化失败 | 不返回新消费成功，Core进入不可授权状态 | 已消费的执行不靠写失败自动撤销 |
| T2-B杀进程/改ledger | 当前普通用户部署不作防御成功承诺 | AT41为边界刻画及防误报测试 |

重启后的恢复只允许安全对账和清理，不恢复执行。允许重试必须新用户意图、新grant、新nonce并明确上次结果未知。

## 七款委托可行性登记

| Agent | A现有证据 | B委托给Broker | C实际OS纳入 |
|---|---|---|---|
| Codex | E1，有覆盖例外 | 未验证，app-server审批不等同替换执行体 | 未验证 |
| Claude Code | E1，版本敏感onFailure | 未验证，Hook不能据此获得原生执行所有权 | 未验证 |
| DSH | E2，guard/tool seam | 候选插件typed recipe，未验证 | 未验证，guard不构成OS边界 |
| 豆包 App 内工作模式 | E0 | 未验证 | 未验证 |
| Reasonix | E2，原生Hook与Extension v2分开 | 候选full-trust扩展tool替换，未验证且需评审 | 未验证，原资料冲突保留 |
| WorkBuddy | E1插件类型，阻止合同未知 | 未验证 | 未验证 |
| ZCode | E1同步Hook合同 | 未验证 | 未验证 |

## 技术依据

进程创建属性、受限token、DACL/句柄与WFP权限依据新增官方资料N01–N16。方案中的Broker流程与事务是本次设计合同，未宣称Windows API实验通过。[来源登记](13-SOURCE-REGISTER.md) [密码学合同](18-CRYPTOGRAPHIC-CONTRACT.md)

## 风险

Broker是新的受信攻击面。把同用户Core再拆进程不会抵御T2-B，Broker接收任意命令或过宽句柄会成为权限代理。执行与授权无法跨OS副作用做到任意exactly-once。

## 尚未确定的内容

D01安全主体、D02网络强制方案、D03 GUI兼容方案、D04首个可委托recipe/Agent、D05密钥与防回退信任锚均等待负责人决定，当前只批准可逆的协议设计与合成验证。[决策登记](P0.1-DECISION-LOG.md)

