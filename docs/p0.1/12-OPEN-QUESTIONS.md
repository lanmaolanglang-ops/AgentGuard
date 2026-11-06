# 开放问题与进入下一阶段的条件

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05

## 目的

列出无法从现有资料确定的关键事项，指定关闭证据、负责人角色和阻塞范围。避免把未证实能力写成已实现。

## 结论

可开展 P1-A 的合成协议、策略、账本和文字原型实现。自动许可、真实 Agent Hook 部署、L2/L3 展示以及面向普通用户的强保护发布，分别受下列 gate 约束。文档交付不等于对七款兼容验收。

## 问题登记

| ID | 问题与现有依据 | 关闭需要的证据 | 责任角色 / Issue | 阻塞范围 |
|---|---|---|---|---|
| Q01 | 七款 client/runtime/adapter/tool provider 的精确构建与签名未映射，桌面包号不足 | 目标版本清单、签名、内核摘要、可重现接口 fixture、官方对应说明 | 兼容负责人，I01/I23 | 每款真实接入与升级认证 |
| Q02 | A04 用户最新指定 App 内工作模式，按Windows目标暂定电脑版，具体包/版本未知 | Windows客户端 About、发布者、模式名称、官方产品入口与开放合同 | 产品/兼容，I13 | 豆包事件源及所有拦截能力 |
| Q03 | Codex timeout/error可继续、write_stdin无重新pre、hosted/专用路径排除 | 每入口独立故障/参数变更/持续输入证据，或更强执行边界 | Codex适配，I10/I07 | Codex故障关闭、强绑定/单次自动许可与相关L3 |
| Q04 | WorkBuddy桌面有Hook插件类型但缺同步verdict合同，不能套用CodeBuddy | 桌面5.9.3等目标构建的官方payload、拒绝/审批/timeout说明，隔离复现 | WorkBuddy适配，I15 | 桌面动态阻止和审批接管 |
| Q05 | Reasonix官网Windows沙箱与固定文档冲突，原生/兼容Hook不同 | 安装版本源码/发布映射、Windows边界负面实验、维护者现有公开说明 | Windows/Reasonix，I14/I23 | Reasonix OS隔离及强审批 |
| Q06 | Reasonix官网v1链接与仓库Extension v2不一致 | 2.56.0实际协议握手/manifest/schema，full-trust权限评审 | 扩展负责人，I14 | Extension部署，禁止盲装 |
| Q07 | ZCode滚动文档无安装版本，timeout最终行为/子Agent继承未知 | 固定构建签名、新session Hook矩阵、错误和超时body sentinel | ZCode适配，I16 | ZCode L2与连续有限授权 |
| Q08 | AppContainer/受限token兼容任意Agent工具链、GUI、COM、loopback未知 | 按实际权限记录的隔离VM资源边界、继承句柄、联网/GUI/工具链回归 | Windows安全，I07 | 通用L3与GUI覆盖声明 |
| Q09 | Hook返回allow不保证最终参数、一次执行或无后续改写 | 最终执行点参与digest+nonce兑换，或明确关闭强授权能力 | 策略/各适配，I06/I10–16 | 防重放与连续授权发布 |
| Q10 | Windows家庭版无默认管理员下强制出站、整机读文件观察有缺口 | 实际OS network capability/原生sandbox验证及sensor权限说明 | Windows安全，I07/I21 | 全量网络/读取审计声明 |
| Q11 | DSH master与本机桌面rc内核未映射，plugin/executor可替换 | 固定release bundle、guard顺序、PTC/preset/卸载测试 | DSH适配，I12 | DSH前置强保护 |
| Q12 | 本地哈希链/密钥/ledger同SID可被攻击者重算或替换 | 独立锚点方案及明确威胁边界，不宣称抗本机管理员 | 审计安全，I08/I20 | 抗篡改宣传，不阻塞普通审计记录 |
| Q13 | 七款事件字段与迟到/丢失/历史位置差异 | 最小脱敏fixture、读取授权范围、稳定source ID、缺失语义 | 协议/兼容，I01/I03/I21 | L1真实观察 |
| Q14 | 用户态Core退出后的host行为、长期进程与Job句柄未知 | UI/Core/Adapter故障注入、restart epoch、in-flight对账 | Core/QA，I22/I17 | 任一fail-closed声明 |
| Q15 | 非技术用户审批理解、浮窗打扰、有限授权默认值未验证 | 可用性研究和严重误判修正，DPI/读屏验收 | 产品/UX，I19 | 面向普通用户发布 |
| Q16 | 实际性能、SQLite durable策略和已知版本缺陷未核 | pinned依赖、busy/满盘/checkpoint、压测与损坏恢复结果 | Rust/QA，I05/I20 | 许可ledger可靠性 |
| Q17 | 插件分发信任、配置CAS、签名与安全卸载未验证 | 可审查变更清单、隔离回滚实测、签名发布包 | 发布负责人，I18/I20 | 真实机器安装 |
| Q18 | 原生审批回退可能走auto/full access、网络审批聚合多请求 | 每profile证明真正等待、精确请求消费、不扩大scope | 各适配/策略，I06/I17 | defer_to_native自动启用 |

## 优先处理顺序

Q01/Q02/Q04/Q05/Q07先收敛身份与合同。Q03/Q08/Q09/Q10/Q11/Q14决定强制边界。Q12–Q18贯穿可靠性、隐私和发布验收。缺少外部接口的产品继续保留在名单中，P1可先交付状态登记，不虚构观察能力。

## 技术依据

未决项直接来自已读官方资料、固定源码和本地元数据的差异。详见来源登记的冲突表。[来源登记](13-SOURCE-REGISTER.md)

## 风险

如果 P1 一开始就把 UI 完整度当作安全完成度，会掩盖强制点不足。若对所有七款承诺同样的防重放/失败关闭，而无法获得最终执行点，该发布目标为 BLOCKED。

## 尚未确定的内容

表内所有问题尚未通过实验关闭。用户确认了A04产品模式方向，但没有提供构建版本。相关实验所需的目标包、隔离环境和真实配置变更均不在本次P0.1执行范围。


## P0.1新增问题映射

| 问题 | 当前状态 | 责任与阻塞 |
|---|---|---|
| Q19 T2-B安全主体 | D01未决，普通同用户Core不能全面防御 | 安全/发布，I07-A/I20-C，阻塞T2-B声明 |
| Q20 网络强制backend与权限 | D02未决，Firewall/WFP/AppContainer均待实验 | Windows，I07-B，阻塞网络OS证书 |
| Q21 GUI隔离与正常功能 | D03未决，独立桌面/UIPI/COM/Clipboard待测 | Windows/兼容，I07-A/B，阻塞GUI声明 |
| Q22 首个B委托路线与recipe | D04未决，七款B均无端到端证据 | Core/兼容，I06/I10–I16，阻塞真实自动许可 |
| Q23 密钥部署与反回退锚 | D05未决，DPAPI仅静态保护 | 安全，I06/I08/I20-C，阻塞抗同SID/回滚声明 |
| Q24 CI接线 | 本地标准验证已完成，无Git仓库且仅Markdown，CI尚未部署 | 协议，I03，阻塞P1-A协议门禁完成 |

Q01–Q18保留为未实验关闭，Q19–Q23在决策日志列方案/推荐/风险与实验。重大决策合计5项，Q24是实现接线工作，不冒充负责人架构选择。每项承诺的真实状态在验收报告中单独列出。

[决策与推荐](P0.1-DECISION-LOG.md) [本地验证与CI合同](20-PROTOCOL-VALIDATION-FIXTURES.md)

