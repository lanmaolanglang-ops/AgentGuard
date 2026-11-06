# P1开发依赖与准入计划

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · AG-01 / AG-08 / AG-10

## 阶段与授权范围

P1-A为纯数据协议、Core/策略/ledger、合成Broker、UI与自动测试。P1-B为隔离环境建设、Windows PoC、真实Agent接入与危险测试。P1-C为独立端到端发布审查、最终供应链与正式发布。当前只交设计，没有实施这些任务。

原24个父Issue全部保留。I02拆成A合成/B真实环境，I07拆成A早期可行性/B完整Broker边界，I20拆成A早期供应链/C最终复核，总计27个执行节点。父Issue只有全部子节点完成才关闭。拆分使隔离先验不依赖完整七款Adapter，供应链基线先于外部工具执行。

下表是唯一依赖源。依赖精确到节点，空列表仅I01。图逐边从同一表生成，不能把任务范围或条件句当依赖。每个依赖必须完成其交付，危险任务另需具体隔离方案审查和实际权限授权。本次计划不授权未来安装/配置或危险操作。

## 执行节点

| 节点 | 阶段 | 责任 | 标题 | 直接依赖 | 交付物 | 验收条件 |
|---|---|---|---|---|---|---|
| I01 | P1-A | 兼容/QA | 身份与证据包 | 无 | 七款client/runtime/provider/binary/config/version/signature清单、脱敏来源包 | 逐款证据可追溯，未知保留L0，A04 App模式明确，WorkBuddy分离 |
| I02-A | P1-A | QA | 合成夹具 | I01 | 纯数据事件、虚拟文件树描述、状态/时间/编码夹具 | 20全部正反边界及迁移通过，无真实资源访问 |
| I02-B | P1-B | Windows/QA | 真实隔离环境 | I01, I20-A | 可回滚VM、无共享凭证/用户目录、合成worker、受控sink和reset记录 | 允许对照可观察，环境审查通过才运行危险测试，不要求先完成Adapter |
| I03 | P1-A | Rust/协议 | 协议与IPC | I01, I02-A, I20-A | schema2.0、parser/role/peer/帧限额、CI接线、迁移和Rust向量验证 | 65夹具及全64状态对通过，AT21/23/29/34，CI真实运行，拒绝Adapter allow |
| I04 | P1-A | 安全/Rust | 分类与硬禁 | I03 | 确定性风险、typed recipe分类、硬禁与unknown处理 | AT01–07/19/26的合成分类通过，未知不自动许可 |
| I05 | P1-A | Rust | SQLite审计与授权账本 | I03 | 唯一writer、durability、nonce UNIQUE、事务/恢复/迁移 | AT21/22/29/36/45合成实现与临时DB故障，消费/审计原子 |
| I06 | P1-A | 安全/Rust | 授权与Execution Broker | I04, I05 | parent rule与单次ticket管理合同、typed Broker/Worker、Prepare/Attest/Redeem/Launch/Cancel/Reconcile，网络代理接口 | 合成AT17–19/22/30/36/39/44/45，Core独占key，最终参数变化拒绝，无通用shell提权RPC，真实B仍需D04与I07-B |
| I07-A | P1-B | Windows安全 | 早期Windows隔离可行性 | I02-B, I20-A | ISO01–07合成worker token/桌面/句柄/网络backend可行性和功能报告 | AT08/09/38/41/42/43初验，D01/D02/D03给出证据，不要求真实Agent，失败无无隔离回退 |
| I07-B | P1-B | Windows/Core | Broker资源边界 | I06, I07-A, I22 | 文件身份/TOCTOU、网络Broker出口、进程树/GUI/COM/句柄边界 | AT08–10/25/28/30/31/38/42–45对应资源全部通过才可给证，backend/权限记录 |
| I08 | P1-A | Rust/安全 | 审计、脱敏与解释 | I03, I05 | audit-v2封链、脱敏、解释模板、回放只读投影、锚点范围 | AT21/24/27/34/40，真实秘密不落日志，回放无执行权，同SID限制公开 |
| I09 | P1-A | UI/UX | 三界面与审批状态 | I03, I06, I08 | 主界面/浮窗/高危弹窗，scope+五flag、grant与执行分列 | AT16/19/32/37/45合成UI状态，七行固定，无最终点禁连续自动许可 |
| I10 | P1-B | 兼容 | Codex适配 | I01, I02-B, I03, I06, I07-A, I23 | Hook/app-server分profile、排除hosted/write_stdin，B委托可行性报告 | AT14/15/17/19/20/33/39逐入口，fail-open故障限制公开，无B证据禁强绑定 |
| I11 | P1-B | 兼容 | Claude Code适配 | I01, I02-B, I03, I06, I07-A, I23 | 同步Hook/onFailure版本矩阵、MCP/子Agent/作用域映射 | 固定构建AT12–20/35/39，文档版不代本机，B独立评估 |
| I12 | P1-B | DSH/安全 | DSH桥接 | I01, I02-B, I03, I06, I07-A, I23 | release bundle映射、Cordis guard/pre/result/PTC桥、插件失载/直接OS范围 | AT12–20/25/28/33，guard不作OS证据，B typed tool替换单独验 |
| I13 | P1-B | 产品/兼容 | 豆包App模式合同 | I01, I02-B, I03, I23 | Windows App内工作模式构建与官方接口证据、可回滚方案或L0缺口报告 | Q02/Q13有明确结论，接口缺失不虚构事件，不套独立豆包工作 |
| I14 | P1-B | 兼容/安全 | Reasonix双路线 | I01, I02-B, I03, I06, I07-A, I23 | 原生Hook与Extension v2分profile、full-trust评审与Windows冲突记录 | AT13–20/25/28，host deny不能提升，未闭冲突关闭相关能力 |
| I15 | P1-B | 兼容 | WorkBuddy桌面 | I01, I02-B, I03, I07-A, I23 | 原生审计导入PoC、桌面同步verdict合同或缺口报告、本地/云分开 | AT20/21/24/35，CodeBuddy结果无效，无阻止合同维持L0/L1实际范围 |
| I16 | P1-B | 兼容 | ZCode用户级 | I01, I02-B, I03, I06, I07-A, I23 | processHook/插件映射、enabled/新session、项目ignored、timeout矩阵 | AT12–20/35/39，固定二进制，未知fault不授故障关闭，B另证 |
| I17 | P1-B | QA/安全 | 独立端到端认证 | I02-A, I02-B, I03, I04, I05, I06, I07-B, I08, I09, I10, I11, I12, I13, I14, I15, I16, I18, I19, I20-A, I21, I22, I23 | AT01–46对应范围报告、独立副作用证据、每款能力证书/排除项 | 所有声明范围无关键SKIP，L1/L2/强绑定/单次/L3独立，未知产品允许交L0报告 |
| I18 | P1-B | 发布/Rust | 安装与配置CAS回滚 | I03, I09, I10, I11, I12, I13, I14, I15, I16, I20-A, I23 | 签名测试包、自有命名空间配置diff、并发CAS、卸载/最低版本策略 | AT35/46隔离通过，无默认代理/证书/PATH/服务变更，保留用户条目与失效旧票 |
| I19 | P1-B | UX/QA | 理解与无障碍 | I09, I02-B | 至少5人构造场景理解、DPI/读屏/键盘与通知报告 | AT32/37，关键误判修正，L2故障限制和L3 Hook失效可理解 |
| I20-A | P1-A | 安全/发布 | 早期供应链基线 | I01 | 依赖来源/lock/SBOM计划、最低版本、签名/FFI审查与PoC探针审查 | 执行外部工具前来源固定，SQLite缺陷/版本核对，禁止未审包进入实验 |
| I20-C | P1-C | 独立安全/发布 | 最终供应链性能复核 | I17, I18, I19, I20-A, I21, I22 | 最终lock/SBOM/签名/降级复核、实际压测、独立安全报告 | AT23/27/29/34/46，真实构建与报告一致，未测性能无承诺，重大决策已处置 |
| I21 | P1-B | Windows/QA | Sensor与覆盖报告 | I01, I02-B, I03, I08, I07-A | 普通用户可得进程/文件/网络observer、权限探测与gap报告 | AT21/24/31/40，无全量读取虚假承诺，不默认新driver/service |
| I22 | P1-A | Rust/Windows | Core独立生命周期 | I03, I05, I06 | UI/Core独立退出、Worker lease/Job所有权、Core epoch、未决对账 | 合成AT16/18/33/36/40/45，无旧permit恢复，真实Job行为由I07-B验证 |
| I23 | P1-A | 兼容/安全 | 来源与版本冲突收敛 | I01 | Reasonix v1/v2/Windows、ZCode scope、WorkBuddy桌面合同、所有来源失效报告 | 固定release依据或明确关闭相关能力，不挑乐观来源，E0–E2不升E3 |
| I24 | P1-C | 安全/产品 | 分级发布准入 | I17, I18, I19, I20-C, I21, I22, I23 | 七行真实状态、证书范围/例外、恢复/卸载、D01–D05处置与发布决定 | 所有声明gate有证据，AT46/签名/回滚通过，未闭关键决策阻止相应发布，禁止七款整体保护宣传 |

## 精确依赖图

节点中的下划线对应表中连字符。共27个节点，图仅显示直接依赖。

<!-- ag:p1-dag -->

~~~mermaid
flowchart TD
  I01
  I01 --> I02_A
  I01 --> I02_B
  I20_A --> I02_B
  I01 --> I03
  I02_A --> I03
  I20_A --> I03
  I03 --> I04
  I03 --> I05
  I04 --> I06
  I05 --> I06
  I02_B --> I07_A
  I20_A --> I07_A
  I06 --> I07_B
  I07_A --> I07_B
  I22 --> I07_B
  I03 --> I08
  I05 --> I08
  I03 --> I09
  I06 --> I09
  I08 --> I09
  I01 --> I10
  I02_B --> I10
  I03 --> I10
  I06 --> I10
  I07_A --> I10
  I23 --> I10
  I01 --> I11
  I02_B --> I11
  I03 --> I11
  I06 --> I11
  I07_A --> I11
  I23 --> I11
  I01 --> I12
  I02_B --> I12
  I03 --> I12
  I06 --> I12
  I07_A --> I12
  I23 --> I12
  I01 --> I13
  I02_B --> I13
  I03 --> I13
  I23 --> I13
  I01 --> I14
  I02_B --> I14
  I03 --> I14
  I06 --> I14
  I07_A --> I14
  I23 --> I14
  I01 --> I15
  I02_B --> I15
  I03 --> I15
  I07_A --> I15
  I23 --> I15
  I01 --> I16
  I02_B --> I16
  I03 --> I16
  I06 --> I16
  I07_A --> I16
  I23 --> I16
  I02_A --> I17
  I02_B --> I17
  I03 --> I17
  I04 --> I17
  I05 --> I17
  I06 --> I17
  I07_B --> I17
  I08 --> I17
  I09 --> I17
  I10 --> I17
  I11 --> I17
  I12 --> I17
  I13 --> I17
  I14 --> I17
  I15 --> I17
  I16 --> I17
  I18 --> I17
  I19 --> I17
  I20_A --> I17
  I21 --> I17
  I22 --> I17
  I23 --> I17
  I03 --> I18
  I09 --> I18
  I10 --> I18
  I11 --> I18
  I12 --> I18
  I13 --> I18
  I14 --> I18
  I15 --> I18
  I16 --> I18
  I20_A --> I18
  I23 --> I18
  I09 --> I19
  I02_B --> I19
  I01 --> I20_A
  I17 --> I20_C
  I18 --> I20_C
  I19 --> I20_C
  I20_A --> I20_C
  I21 --> I20_C
  I22 --> I20_C
  I01 --> I21
  I02_B --> I21
  I03 --> I21
  I08 --> I21
  I07_A --> I21
  I03 --> I22
  I05 --> I22
  I06 --> I22
  I01 --> I23
  I17 --> I24
  I18 --> I24
  I19 --> I24
  I20_C --> I24
  I21 --> I24
  I22 --> I24
  I23 --> I24
~~~

## 安全准入

P1-A可从I01开始，随后I02-A/I20-A/I23，I03接通CI后才继续相应实现。合同验证通过不计Core/Rust实现通过。A04、WorkBuddy等无接口时交付明确L0/缺口报告，不伪造适配完成。

P1-B先I02-B，再I07-A用合成worker验证token/GUI/网络候选。D01–D03未选择时可比较候选，但没有默认service、Firewall/WFP或真实Agent变更授权。I07-B需要最终兑换和生命周期实现完成。每款L3另需该资源范围I07-B及I17证据，A有限deny认证可独立推进。

P1-C当前BLOCKED。I17实际认证、I18/46签名更新与回滚、I19理解、I20-C复核及D01–D05对发布范围的处置均未完成。允许某款保持L0并明确公布，但不得称七款完整保护通过。D01等未决不阻止无该声明的合成工作。

## 完成定义

每节点有审查diff、固定来源/构建/依赖、正反与故障证据、AT编号、恢复路径和未覆盖项。I17报告需区分接口E1/E2、合成通过、实际E3入口、E4资源边界。CI输出和真正OS实验分开。I24以发布包hash绑定所有报告，更新失配重新认证。

[执行代理](15-EXECUTION-BROKER-CONTRACT.md) [测试计划](10-TEST-AND-ACCEPTANCE-PLAN.md) [决策](P0.1-DECISION-LOG.md) [协议验证](20-PROTOCOL-VALIDATION-FIXTURES.md)

