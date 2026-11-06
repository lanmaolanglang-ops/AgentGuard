# 确定性安全策略与有限授权

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05

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

parent rule与单次ticket分开。parent rule只表达经过明确批准的会话、类别、目录、期限和最大次数，每次操作仍发行一个max_uses=1的grant/permit。高风险默认单次人工，critical命中硬禁deny。

单次grant正式字段、permit绑定字段与OperationSnapshot以07为准。核心包含grant_id、reservation_id、key_id、nonce_hash、core_boot_epoch、policy/grant epoch、operation_digest、broker/prepared/invocation、agent/runtime/adapter/session、requestor/executor principal与expiry。不得把parent的20次上限写进单次permit.max_uses。

parent rule、用户批准快照与有限次数ledger是Core内部管理记录，尚未开放外部通用规则API，I06需在实现前按同样闭合原则定义管理Schema。原10分钟/20次/最长1小时仅P0产品提案，当前未采纳为安全默认值，等待I19与负责人审查。

连续自动许可需要B真实委托、parameter_binding_verified和single_use_grant_verified均true、原生allow、有效精确profile、完整最终数据及该recipe所需OS边界。不能使用level>=L2替代这些条件。[能力规则](16-PROTECTION-CAPABILITY-CONTRACT.md)

## 完整操作摘要

operation_digest按18的operation-v2域编码，先计算不可变semantic快照，再绑定nonce/invocation/prepared与epoch，最后签permit。Core独占全部HMAC密钥，Broker只在线兑换。确定性字节格式、secret表示、Unicode、路径和长度边界均以18为唯一规范，禁止自行JSON.stringify后签名。

任何安全字段缺失进入unknown并关闭强许可。原始权威操作与审计脱敏副本分开，不能从“***”重建执行。语义变化使旧批准失效，fresh invocation必须重新绑定。

延后意图默认关闭。确需重提议时，必须保留同一prepared身份/对象并原子认领一次，Worker重建或语义改变重新审批，A无最终点时禁用。D04未关闭前只保留显式deny与原生真实人工确认。

[字节、密钥、Nonce和21向量](18-CRYPTOGRAPHIC-CONTRACT.md) [OperationSnapshot与permit](07-UNIFIED-EVENT-SCHEMA.md)

## 路径与 TOCTOU

对既有文件通过 handle 检查 volume/file ID、最终路径、reparse 链及访问种类。默认拒绝未审查 reparse、junction、设备路径、UNC、ADS、非 NTFS 或不明挂载关系。大小写、8.3 别名、尾随点/空格、硬链接和替换目录都纳入测试。

对新建目标验证父目录 handle 与 leaf 名。可控 broker 尽可能使用已验证对象身份执行，并复核替换竞态，避免关闭 handle 后按原文本重新打开。Windows API 与工具兼容细节需 P1 证明，不能把词法 startsWith(root) 当目录边界。

任意 shell 的后续 fopen、符号链接变化、脚本二次读取无法由一次 pre-hook 全部锁住。强保证要求执行环境对整个进程树实施资源限制，或把敏感操作缩小到经过审查的 broker。无法满足时不授予泛化目录写/删除能力。

## 授权状态机

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

协议枚举为小写。created由Core接收新授权请求，pending表示等待明确用户意图，reserved在确认和额度预占事务提交后产生，consumed仅由最终Redeem事务产生。撤销/到期/上下文失效/准备失败分别进入revoked/expired/invalidated/failed。所有终态不可转回，重复查询不追加第二次状态。

Execution单独使用prepared、launch_authorized、running、succeeded、failed、cancelled、unknown。consume之后执行失败仍保持grant.consumed，取消在途只更新执行记录。全部合法与非法状态对由20本地验证，实际并发由AT45验证。

## 决策流程

~~~mermaid
flowchart LR
  Ingress --> Parse
  Parse --> HardDeny
  Parse --> ModeA
  ModeA --> NativeDeny
  ModeA --> DeferNative
  Parse --> ModeB
  ModeB --> NativePermission
  NativePermission --> RequestorIdentity
  RequestorIdentity --> Classification
  Classification --> Prepare
  Prepare --> Attest
  Attest --> UserIntent
  UserIntent --> Reserve
  Reserve --> Redeem
  Redeem --> Launch
  Launch --> ExecutionEvidence
~~~

图中分支实际由严格校验控制，任一缺证据返回deny/ask，无默认进入Launch。HardDeny优先，NativePermission未知也禁止B。模式C为独立OS认证，不能代替精确授权。

## 并发、重试与防重放

设计要求在 ledger 事务中完成 pending request 校验、grant epoch 核对、额度预占与 decision audit。最终执行点以 unique nonce 在同一许可账本上原子消费。N 次并发兑换只能有一个成功，expired/revoked/不同 audience/digest 均拒绝。

Core 与受控执行器必须定义唯一的授权线性化点。撤销在消费之前提交，则禁止启动。消费之后撤销属于 in-flight，尝试取消/终止受控执行树并记录结果，不能承诺已写文件或已发网络请求可逆。

执行器消费 nonce 后在进程启动前崩溃会损失一次额度，重启后记 unknown，不重发 permit。这提供可证明的最多一次授权消费，不承诺任意外部工具 exactly-once 成功。

如果原生 host 无 nonce 兑换、最终输入快照或可靠 invocation 幂等，则返回 allow 后的重复执行仍可能发生。该 profile 禁止连续有限授权与自动重放批准，并降为有界前置拒绝能力，强防重放 gate 不通过。

## 时效、撤销与持久任务

Core 使用当前 boot 的 monotonic deadline，wall clock 后跳不延长授权，休眠恢复重新核对。重启core_boot_epoch使旧未消费票invalidated，parent rule重新确认，已consumed仍保留。SQLite 状态被回滚时本地 epoch 可能一起回滚，普通用户威胁模型不宣称抵抗同用户持久状态重放。

撤销立即使未消费许可失效。持久 shell 的 write_stdin 是新增行为载体，需把输入流/脚本作为新操作授权。Codex 这一路径不重新触发前置 Hook，必须禁自动许可或使用额外受控 transport/OS 边界。已批准 npm/python 父命令内部产生的子进程也不自动获得无限授权。[S01](https://learn.chatgpt.com/docs/hooks)

## 失败关闭和原生审批回退

| 条件 | Core verdict | 允许的回退条件 |
|---|---|---|
| 未知命令/缺参数 | ask/deny | 原生明确等待用户，用户看见完整操作，且未越硬禁 |
| 命中硬禁 | deny | 无回退 allow |
| UI 不可达 | pending 到期 deny | 仅已验证原生提示路径可继续询问 |
| Core 不可达/解析错误/账本不可写 | 不产生 permit | B不启动，A按host真实故障记录并停止相关自动许可 |
| Adapter 无法启动 | 无 verdict | 按产品实测，相关fault范围pre flag失效，健康显式deny可保留有限L2并披露故障限制 |
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


## 事务与结果恢复规范入口

消费事务是唯一授权线性化点，撤销先提交禁止消费，消费先提交只能取消在途。响应丢失、Core/Broker崩溃和unknown恢复不自动退款或重发启动授权。对任意OS/远端副作用没有exactly-once事务。[执行与故障合同](15-EXECUTION-BROKER-CONTRACT.md)

