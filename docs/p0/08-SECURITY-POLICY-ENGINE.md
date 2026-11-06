# 确定性安全策略与有限授权

AgentGuard V2.0 · P0-7 · 2026-11-05

## 目的

设计风险分类、硬禁、白名单、有限授权、撤销、参数一致性与失败关闭，并明确可证明范围。

## 结论

Core 独立确定性决策，优先级为硬禁 → 身份/边界可验证性 → 原生更严格限制 → 操作分类 → 有限授权 → 人工确认或拒绝。AI 解释没有授权权。

授权绑定最终操作和执行主体。只有最终执行点参与一致性验证与一次性兑换，才可承诺强参数绑定和防重放。普通 Hook 只能建立受管请求级控制，其后的改写、旁路和 host fail-open 必须通过更强接口或 OS 边界解决。

## 风险和规则

| 等级 | 触发示例 | 决策 |
|---|---|---|
| low | 已审查只读 recipe，真实目标在允许根内、无动态代码/联网、身份与边界有效 | 精确白名单或有限 read grant 内可自动允许 |
| medium | 专用输出目录内的有界创建/修改、可预览且无敏感数据 | 任务授权覆盖可允许，缺条件则 ask |
| high | 批量删除、依赖安装、上传、执行变化脚本、访问项目外 | 每次明确人工确认，必要时隔离后执行 |
| critical | 凭证窃取、持久安全弱化、根目录破坏、授权账本篡改、提权绕过 | 命中硬禁直接 deny，普通审批不覆盖 |
| unknown | 编码/动态脚本、未知工具、目标/身份/执行上下文缺失 | 不自动 allow，可预览补证，否则 ask 或 deny |

风险不只依赖命令名。读取用户私钥是高敏资产访问，不能因“只读”降成 low。git/npm/python/PowerShell 本身均不能作为无条件可复用白名单。

硬禁为产品确定性基线，包含访问或外传机密凭证、关闭或篡改防护、修改 AgentGuard 许可/日志/可执行物、绕过审批、破坏系统和未纳入授权的真实生产资源。规则修改走单独策略管理流程，要求明确变更审计，不能由当前危险请求顺带修改。

若组织或原生 host 已 deny，AgentGuard 只允许维持/收紧。Reasonix full-trust allow 可覆盖 host deny 的能力在本产品中禁止使用。[S25–26](13-SOURCE-REGISTER.md)

## 命令分析

第一版只为少量审查过的结构化操作和 execution recipe 提供自动授权，原始 shell 默认 conservative。PowerShell AST 可帮助解析，但模块、函数、profile、alias 和动态求值仍影响实际语义。解析不等于能预测全部效果。

CMD → PowerShell、Python subprocess、npm scripts、pipeline、重定向、EncodedCommand、Invoke-Expression、shell -c、动态导入都提高风险或进入 unknown。静态脚本可展示内容与摘要，但脚本依赖、下载内容和运行期数据变化仍需执行边界控制。

工作目录、解释器实际文件、shell profile/login 行为、PATH 搜索、COMSPEC、PATHEXT、模块/脚本解析上下文参与操作上下文。环境只采集允许名单中的相关值或摘要，不读取/输出原始凭证。无法冻结环境则该请求不具备强参数绑定条件。

## 授权对象

| 字段 | 约束 |
|---|---|
| grant_id / version | 随机不可预测 ID，Core ledger 管理 |
| agent_id / runtime_digest / adapter_digest | 限定精确产品、运行时和适配器 |
| session_id / subagent scope | 限定会话，默认不自动传给子 Agent 或外部 backend |
| principal | Windows SID、logon LUID、完整性级别、token 限制、process_instance |
| operation_type / tool_provider | 指定操作类型、工具 schema 与 provider identity |
| targets | 目录/文件 identity、读写删分类、目标 host/port/protocol |
| argument_constraints | 精确参数或审查过的 typed 参数范围，不使用任意 shell 前缀代替语义 |
| valid_from / expires_at | 最短必要时限，Core 同时使用 monotonic duration 与 wall clock |
| max_uses / used_uses | 至少 1，单次授权必须为 1，消费事务序列化 |
| policy_epoch / grant_epoch / boot_epoch | 策略更新、撤销和重启使旧许可失效 |
| operation_digest | 单次请求绑定完整不可变操作 |
| decision_record | 用户看到的快照摘要、决定、规则来源及证据 |
| revocation | 状态、时间、原因、传播确认 |

目录授权要求操作类别、时限、次数和会话同时受限。高风险 shell 默认只提供一次，低/中风险的结构化操作才考虑连续有限授权。初始提案为默认 10 分钟、最多 20 次、最长 1 小时，均须用户明确填写并经 Core 上限校验。数字是产品提案，尚未做可用性验收。

## 完整操作摘要

operation_digest 为域分离 HMAC-SHA-256，输入采用协议冻结的确定性 typed 编码。禁止 JSON 重复键，保留字符串精确内容、null/空值区别和数组顺序，不为匹配擅自 trim、lowercase 或重排命令。UTF-8/UTF-16 边界显式编码，无法可靠转换的输入拒绝授权。

摘要绑定以下内容。

- agent/runtime/adapter/tool schema/provider/generation。
- 原始 command、完整 typed arguments、脚本/可执行体摘要、解释器真实路径与文件 identity。
- 工作目录真实 identity、路径作用域、操作类别、相关文件/patch 的内容摘要。
- 执行 SID、logon session、token/integrity、process instance、sandbox 实现和有效策略摘要。
- 相关非秘密环境和秘密参数的 HMAC 表示、stdin payload、shell/profile/login 选择。
- 网络目的地、方法、端口、上传数据摘要及每跳重定向约束。
- policy/grant/boot epoch、source invocation、nonce、有效期限及允许次数。

字段变化导致新摘要，旧批准失效，UI 对比变化并要求重新提议。若某产品不提供其中必要的安全字段，该字段标 unknown，相关强授权能力关闭，不能把 null 视为通配符。

实际许可使用短期 MAC、nonce、audience、expiry、digest，授权 ID 和审计摘要本身不具备执行权。密钥仅 Core/受控执行器持有，不能放入环境变量或日志。普通 Hook 返回的 allow 是宿主协议结果，不能自动升级为可兑换的能力令牌。

### 拒绝后重新提议的绑定

审批超过Hook预算时，原invocation已拒绝并结束。它的operation_digest和permit不能授权下一次调用。Core另保存一次性approved_intent，其intent_digest仅绑定上面列出的完整语义内容和执行上下文，排除每次都会变化的invocation、nonce与事件时间。其余安全字段仍必须完全匹配。

UI必须明确写“本次请求已拒绝，允许下一次相同操作”，用户确认该一次性意图后，新的invocation才能原子认领它。Core再次验证语义摘要、文件内容、身份、策略epoch和到期状态，再用新invocation及新nonce生成全新的operation_digest和permit。意图认领和额度预占同一事务，认领后不自动释放或反复重用。

该流程只在最终执行点具备可靠一次性消费的profile启用。没有该能力时，重新提议继续使用人工或经验证的原生审批，不从后台待办直接自动allow。AT17/19/22/30必须覆盖新旧invocation、双连接同时认领、参数变化与认领后崩溃。

## 路径与 TOCTOU

对既有文件通过 handle 检查 volume/file ID、最终路径、reparse 链及访问种类。默认拒绝未审查 reparse、junction、设备路径、UNC、ADS、非 NTFS 或不明挂载关系。大小写、8.3 别名、尾随点/空格、硬链接和替换目录都纳入测试。

对新建目标验证父目录 handle 与 leaf 名。可控 broker 尽可能使用已验证对象身份执行，并复核替换竞态，避免关闭 handle 后按原文本重新打开。Windows API 与工具兼容细节需 P1 证明，不能把词法 startsWith(root) 当目录边界。

任意 shell 的后续 fopen、符号链接变化、脚本二次读取无法由一次 pre-hook 全部锁住。强保证要求执行环境对整个进程树实施资源限制，或把敏感操作缩小到经过审查的 broker。无法满足时不授予泛化目录写/删除能力。

## 授权状态机

~~~mermaid
stateDiagram-v2
  [*] --> Proposed
  Proposed --> Rejected: 硬禁 / 身份无效
  Proposed --> AwaitingApproval: 无有效有限授权
  Proposed --> Authorized: 有效精确授权
  AwaitingApproval --> Rejected: 拒绝 / 到期 / 变化
  AwaitingApproval --> Authorized: 用户确认匹配摘要
  Authorized --> Reserved: 原子预占一次额度
  Reserved --> Consumed: 最终执行点校验并兑换
  Reserved --> Invalidated: 过期 / 撤销 / 上下文变化
  Consumed --> Running: 有启动证据
  Consumed --> Unknown: 崩溃 / 启动结果不明
  Running --> Succeeded: 成功证据
  Running --> Failed: 失败证据
  Running --> Unknown: 结果丢失
  Authorized --> Invalidated: 策略或身份变化
  Rejected --> [*]
  Invalidated --> [*]
  Succeeded --> [*]
  Failed --> [*]
  Unknown --> [*]
~~~

Reserved 消耗可用额度。无执行证据或崩溃时不自动退回次数，避免模糊结果重试导致重复执行。重新执行需重新提议，用户可看到“上次是否执行未知”。

## 决策流程

~~~mermaid
flowchart TD
  I["提议与原始上下文"] --> V{"版本、身份、schema 有效"}
  V -->|否| D["拒绝 / 降级"]
  V -->|是| H{"硬禁命中"}
  H -->|是| D
  H -->|否| N{"原生限制允许且最终执行边界可证"}
  N -->|否或缺失| X["禁自动许可，补证或可靠原生审批"]
  N -->|是| C["分类与完整摘要"]
  C --> G{"有限授权完全匹配"}
  G -->|是| L["事务预占与审计"]
  G -->|否| U{"可安全展示并等待用户"}
  U -->|否| D
  U -->|是| A["人工有限确认"]
  A --> R{"再次核对摘要与撤销 epoch"}
  R -->|变化| D
  R -->|匹配| L
  L --> F{"最终执行点原子兑换与参数一致"}
  F -->|否| D
  F -->|是| E["启动受限操作，持续记录结果"]
~~~

## 并发、重试与防重放

设计要求在 ledger 事务中完成 pending request 校验、grant epoch 核对、额度预占与 decision audit。最终执行点以 unique nonce 在同一许可账本上原子消费。N 次并发兑换只能有一个成功，expired/revoked/不同 audience/digest 均拒绝。

Core 与受控执行器必须定义唯一的授权线性化点。撤销在消费之前提交，则禁止启动。消费之后撤销属于 in-flight，尝试取消/终止受控执行树并记录结果，不能承诺已写文件或已发网络请求可逆。

执行器消费 nonce 后在进程启动前崩溃会损失一次额度，重启后记 unknown，不重发 permit。这提供可证明的最多一次授权消费，不承诺任意外部工具 exactly-once 成功。

如果原生 host 无 nonce 兑换、最终输入快照或可靠 invocation 幂等，则返回 allow 后的重复执行仍可能发生。该 profile 禁止连续有限授权与自动重放批准，并降为有界前置拒绝能力，强防重放 gate 不通过。

## 时效、撤销与持久任务

Core 使用当前 boot 的 monotonic deadline，wall clock 后跳不延长授权，休眠恢复重新核对。重启 boot_epoch 全部旧许可失效，可复用 grant 也要求重新确认。SQLite 状态被回滚时本地 epoch 可能一起回滚，普通用户威胁模型不宣称抵抗同用户持久状态重放。

撤销立即使未消费许可失效。持久 shell 的 write_stdin 是新增行为载体，需把输入流/脚本作为新操作授权。Codex 这一路径不重新触发前置 Hook，必须禁自动许可或使用额外受控 transport/OS 边界。已批准 npm/python 父命令内部产生的子进程也不自动获得无限授权。[S01](https://learn.chatgpt.com/docs/hooks)

## 失败关闭和原生审批回退

| 条件 | Core verdict | 允许的回退条件 |
|---|---|---|
| 未知命令/缺参数 | ask/deny | 原生明确等待用户，用户看见完整操作，且未越硬禁 |
| 命中硬禁 | deny | 无回退 allow |
| UI 不可达 | pending 到期 deny | 仅已验证原生提示路径可继续询问 |
| Core 不可达/解析错误/账本不可写 | 不产生 permit | host 实测 timeout/error 必须拒绝，不能靠 UI 降级补救 |
| Adapter 无法启动 | 无 verdict | 按产品实测，fail-open 则 L2 失效，强保护不能上线 |
| 原生自动许可/full access | 不能当人工回退 | 需独立可靠强制点，否则自动授权禁用 |
| 外部观察模式 | not_evaluated | 不显示“已阻止”，只能风险提示 |
| OS 边界创建失败 | deny，终止未执行进程 | 禁止自动退回不隔离执行 |

Codex 的错误可继续、Claude 的显式 onFailure block、Reasonix 原生 timeout/spawn 差异、ZCode 未知 timeout，必须分别验收。Core 自己的 fail-closed 代码不能改变宿主 fail-open。[S01、S07、S24、S35](13-SOURCE-REGISTER.md)

## 中文解释与证据

解释由本地模板先生成。输出四栏为“已验证事实”“Agent 给出的理由”“根据命令推断”“未知”。模型只能增加 explanation，并携带 model/version/input_redaction，不能更新 risk/decision。

删除 cache 的例子应写“准备递归强制删除该目录，通常不经回收站。实际文件数量及重要性尚未确认”，并区分已经解析的 -Recurse/-Force 与未验证的文件范围。不能因目录名 cache 推定可安全删除。

## 技术依据

前置生命周期及修改/失败语义见各 Agent 官方合同。令牌/ACL/Job/AppContainer 的资源边界、SQLite 单写事务与 IPC 身份构成执行基础。[能力调查](04-AGENT-CAPABILITY-MATRIX.md) [信任边界](03-TRUST-BOUNDARIES.md) [S40–45、S54](13-SOURCE-REGISTER.md)

## 风险

最终执行点不合作时，参数、次数和 fail-closed 只能限受管请求成立。用户批准后的 in-flight 操作不可保证撤回。任意 shell 语义不可完全静态判定，已授权脚本仍需隔离。

## 尚未确定的内容

每产品是否提供最终输入/执行身份/nonce 消费点，重试和审批聚合的幂等性，Windows handle-based broker 的实现细节。上述为 P1-I04/I06/I07/I10–I16 的发布 gate。

