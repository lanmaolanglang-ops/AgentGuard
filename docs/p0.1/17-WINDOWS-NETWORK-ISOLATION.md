# Windows 网络安全与出站控制

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · AG-03 · 待实验设计

## 目的

区分网络观察、请求代理与OS强制阻断，定义授权结构、旁路、现有连接及P1实验。当前没有网络隔离实现或网络实验结果。

## 结论

普通代理变量、域名匹配和Agent Hook只能影响合作路径。AppContainer的Internet类capability表达广泛资源能力，不能单独表达逐域名白名单。Firewall/WFP可能提供强制控制，但管理权限、过滤层、现有规则和产品兼容性必须验证，当前作为候选方案。

P0.1不承诺全面域名白名单隔离。推荐优先实验“无直接网络能力的Worker + 显式typed Network Broker”，仍需证明Worker无直连、loopback代理和间接进程通路。最终选择见D02，未替项目负责人决定系统策略或管理员部署。

## 三类能力

| 类别 | 可以证明什么 | 无法据此证明什么 |
|---|---|---|
| observation | 观察到请求、连接或某次sink接收 | 未观察到流量等于没有流量，或所有旁路已阻止 |
| request_proxy | 经本Broker的请求满足方法/目标/数据限制 | 任意Host、浏览器、子进程和MCP只能经Broker联网 |
| os_enforced | 经过实际验证的OS强制点阻止所列进程和协议越界 | 域名/HTTP语义天然完整，或所有已建立流量立即终止 |

协议network_requests[].enforcement_state使用none、request_proxy、broker_egress、os_verified、unknown。表中os_enforced是机制类别，正式状态名为os_verified，Adapter不能自行提升。os_isolation_verified证书另列网络维度和实际backend。

## 候选Windows机制

| 机制 | 真实作用 | 权限/部署条件 | 局限与当前状态 |
|---|---|---|---|
| AppContainer网络能力 | OS根据有效token、网络隔离策略限制Internet/私网/server等资源 | 在受控创建路径设置，系统服务/策略、loopback例外等影响有效行为 | internetClient不是域名ACL，缺能力场景须测IPv4/6、UDP等，待实验 |
| LPAC | 进一步收紧ALL APPLICATION PACKAGES等默认资源访问 | 自建受控进程与所需capability、对象DACL，兼容性待测 | COM/注册表/工具链易失效，不能当现成Agent部署方案 |
| 未打包Win32同用户Host | 默认持有用户网络能力，可直接Winsock/系统代理/外部进程 | 普通用户态包装器不能自动剥夺既有Host全部网络权 | 受限token本身不等于网络白名单，需独立边界 |
| Windows Firewall | OS规则匹配应用、地址、协议、方向和配置profile | 修改系统防火墙策略通常需管理员或现成受委派权限 | 显式block优先于冲突allow，规则交互/IPv6/现有流量需测，待实验 |
| WFP用户态管理 | 对BFE对象、filter/layer等实施管理 | OPEN不等于ADD，需FWPM_ACTRL_ADD/ADD_LINK等对象权限 | 可以复用系统过滤而无需自研callout驱动，具体层/协议覆盖与授权待实验 |
| 自研WFP callout | 可扩展内核分类逻辑 | 驱动、签名、管理员部署和维护 | 超出第一版默认约束，未选择 |
| Network Broker | 对收到的typed请求执行DNS/TLS/HTTP与数据约束 | 普通用户可代理自己的请求，Broker连接身份和出口仍需保护 | 不能单独强迫Host经过它，当前只作为待实现请求代理 |
| MXC/Agent原生sandbox | 某构建可有独立网络强制能力 | 依赖实际Windows补丁、运行实现与策略 | 既有公开资料不证明本机有效，仍需独立实验 |

WFP文档允许everyone打开/分类部分对象，但添加过滤需要其它访问权，不能以FwpmEngineOpen成功认定普通用户可部署完整网络隔离。Firewall的通配路径和规则优先级也不能凭经验假设。[N02](https://learn.microsoft.com/en-us/windows/win32/fwp/access-control) [N05](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/rules)

## 出站模型

~~~mermaid
flowchart LR
  Host --> NativeNetwork
  Worker --> RequestIPC
  RequestIPC --> NetworkBroker
  NetworkBroker --> Core
  NetworkBroker --> Resolver
  NetworkBroker --> RemoteSink
  DirectAttempt --> OSBoundary
  OSBoundary --> DeniedPath
  NativeNetwork --> CoverageGap
~~~

Host→NativeNetwork表示尚未受控的旁路，不能画成已被本产品阻止。只有实验验证Worker及所有相关后代没有其它出口，才把其范围标记为broker_egress。外部MCP服务器和模型服务的网络自主行为不属于本地Worker的OS隔离范围。

### 网络授权数据结构

正式NetworkAuthorization位于事件Schema的network_authorization定义，以下字段均参与semantic_digest。

| 字段 | 安全含义 |
|---|---|
| authorization_id / scope_id | 单次网络意图及认证范围 |
| request_method / scheme / host / port | 精确方法与ASCII主机、端口，拒绝未审查user-info、空主机和歧义URL |
| destination_ip / dns_answers / dns_epoch / dns_expires_at | 本次解析及IP集合，实际连接目标必须在经核验集合内 |
| path_query_digest | 路径与query用HMAC表示，默认日志不保存敏感原文 |
| transport | tcp/udp/quic/unknown，recipe明确允许的传输 |
| proxy | none/explicit/broker/unknown、proxy_id及endpoint摘要，禁止从环境自动继承未绑定代理 |
| redirect_chain / redirect_limit | 每跳主机/IP/方法/数据摘要，跨目标重新授权，禁止自动带敏感Authorization |
| upload_data_digest / request_body_digest | 对实际发送字节计算域分离HMAC，正文缺失不能当空正文 |
| max_request_bytes / max_response_bytes / max_requests | 有界数据量和请求次数 |
| connection_id / pool_scope / connection_policy_epoch | 连接池、HTTP2/3多路复用和持久连接绑定，跨授权复用需核验 |
| observation_state / enforcement_state / enforcement_scope_id | 分别表达请求/连接证据与OS/backend证明 |
| native_permission | 原生限制必须允许，不得用Broker越过原生deny或未知许可 |

网络授权不是域名字符串白名单。若实际地址、正文、method、proxy、重定向或执行身份变化，原ticket失效。只读GET也可能带副作用，风险按实际目标及请求语义确定。

## DNS、TLS与HTTP行为

Broker使用已登记的解析器/网络recipe，取得完整A/AAAA结果，拒绝未授权的私网、link-local、multicast、loopback及IPv4-mapped-IPv6等地址形式。域名采用ASCII A-label，不隐式执行版本漂移的IDNA转换。URL的大小写、百分号、路径和query解析采用固定profile，raw URL差异保留。

验证后必须连接该次被批准的IP，TLS仍验证批准host的证书与SNI，禁止再次隐式解析改变目标。DNS rebinding、CNAME结果变化、TTL过期与连接重建需要重新核验。IP授权不能单独证明远端应用身份，TLS验证不关闭。

每个HTTP重定向都作为新跳检查，跨host/port/scheme或正文/方法变化默认暂停重新授权。禁止对307/308等重定向无条件重发敏感正文。日志只保存脱敏目标或HMAC，认证header和query秘密不得进入审计。

CONNECT通道、任意WebSocket、无限streaming和未经审查的协议不作为首个自动recipe。HTTP2连接合并、HTTP3/QUIC、Alt-Svc与WebSocket升级必须禁用或在绑定合同中明确限制，不能靠一次host检查后放任持续隧道。

## 旁路与检查表

| 旁路 | 必须验证的行为 | 当前结论 |
|---|---|---|
| IPv4/IPv6/IPv4-mapped地址 | 全族策略一致，不能只测IPv4 | 未验证，AT43 |
| UDP/QUIC | 直接UDP及QUIC不绕过，仅允许已审查recipe | 未验证，AT43 |
| DoH/DoT/自带resolver | Host自发加密DNS仍走直接出口限制 | 未验证，不依赖拦DNS域名 |
| localhost / loopback | IPv4/6 localhost、named local relay、现有loopback例外 | 未验证，不默认全开loopback |
| 代理绕过 | 无代理、改环境、直连IP、PAC/系统代理差异 | 未验证，代理变量仅合作配置 |
| 子进程/COM/Shell间接激活 | token/网络边界实测，既有外部进程不自动纳入 | 未验证，AT31/42/43 |
| 已建立/持久连接 | 策略撤销后连接与池行为，连接前后时序 | 未验证，不能声称撤销即断所有连接 |
| 继承/重复socket句柄 | Worker不得获得已有连接句柄，DuplicateHandle与IPC无句柄越权 | 未验证，AT44 |
| MCP/浏览器/模型请求 | 各服务/Host的独立流量范围明确 | 本地Worker不能代表远端全链路 |
| 受控Broker本身被攻击 | 输入限长、域隔离、native许可、配额、T2-B边界 | 设计限制，未实现 |

## 撤销与已建立连接

未消费网络ticket立即拒绝。已消费请求记录取消意图，Broker关闭自己持有的连接/stream并报告实际结果。OS policy是否能阻断已经建立连接需backend专项实验，不能从新连接阻止推导。

已经发出的字节、远端执行或缓存无法撤回。连接池按授权scope和epoch隔离，撤销后不得给新请求复用。取消ack仅说明收到请求，独立sink的接收时间/字节数才用于判定实际副作用。

## P1网络隔离实验

NEXP01记录OS build/补丁、AV/Firewall/BFE有效状态、账户权限、policy digest与既有例外，仅在隔离VM读取或配置。NEXP02测试无capability/AppContainer/LPAC的A/AAAA/TCP/UDP/QUIC。NEXP03测试localhost转发与已有socket。NEXP04在明确管理员测试权限下验证Firewall或WFP候选，不在用户机器修改策略。NEXP05验证typed Broker DNS重绑定、每跳重定向、正文HMAC、连接池和撤销。NEXP06验证子进程/GUI间接出口。

以上全部未执行，归属I07-B及AT10/31/43/44。每项先有允许对照、再做拒绝测试，必须同时取得调用到达强制点和独立sink无越界副作用证据。任意关键路径SKIP都阻止该范围network OS证书。

## 技术依据

AppContainer概念与实现文档支持资源能力模型，UWP连接排障说明WFP与loopback处理，但不能直接推定所有未打包Win32行为。新增Windows资料只建立E1事实，不建立E4设备结果。[N01–N06](13-SOURCE-REGISTER.md)

## 风险

强制网络边界可能破坏模型访问、开发server、SSH、GUI联动和插件通信。未经owner批准不能通过自动开Firewall例外、loopback exemption或新增service解决兼容问题。

## 尚未确定的内容

最终backend、管理员初始化需求、可验证协议集合、MXC适用性和既有连接语义均为D02/P1-B待决。当前只提供请求代理合同与实验计划。

