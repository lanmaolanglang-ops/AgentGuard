# I06-D 纯数据合同设计

G0 负责人结论为 **ACCEPTED WITH LIMITATIONS**。本阶段从 `8c2a026468e05b352e4e3ed9de1db3dcc3ce8743` 开始，I05 基线为 `6f9bcf50ce5b327dc0225f612b5982990fe1716a`。冻结依据为 [07 Schema](../../p0.1/07-UNIFIED-EVENT-SCHEMA.md)、[15 执行合同](../../p0.1/15-EXECUTION-BROKER-CONTRACT.md) 和 [18 密码学合同](../../p0.1/18-CRYPTOGRAPHIC-CONTRACT.md)。[G0 验收对应的 Windows CI](https://github.com/lanmaolanglang-ops/AgentGuard/actions/runs/42229482617)

| 对象 | 所有者与约束 |
|---|---|
| Parent Rule | 合成 UI 确认一份具体规则和 Prepared 范围，Core 登记、撤销并检查版本、时间和余额。范围精确包含 Agent、runtime、provider、adapter、config、请求及执行主体、session、recipe、工作目录及目标。用途和预算必须显式给定。 |
| Prepared | 合成 Broker 提交当前决策、snapshot、连接和 Broker／Worker 实例、generation、双时限。Core 验证后冻结，安全字段变化由匹配上下文复核或由 Core 显式注销。 |
| Grant | 绑定单次人工确认、固定决策、固定 Prepared、invocation、可选规则引用和三种 epoch。状态和失效原因留作对账。 |
| Reservation | 不可变。绑定 Grant、Decision、Approval、Prepared、连接与两个实例、语义和操作 MAC、manifest 摘要、nonce 哈希、key ID／generation、OS boot、三种 epoch 和双时限。 |
| Permit | 仅确认的合成预占返回一次冻结 `permit-v2`。验证和消费返回独立的合成记录结果。查询仅返回状态。 |

每次 issue 前都需要 UI 显式确认单次意图。已登记的规则仅约束范围、时间与额度。硬禁、原生 deny、原生未知、身份未知和安全属性未知均拒绝。合成角色仅用于检验所有权及调用顺序。

`created → pending → reserved → consumed`。前三态分别可转 `revoked`、`expired`、`invalidated` 或 `failed`。终态不回退。额度、Reservation ID 和 nonce 哈希在一个独占可变借用内登记，所有会返回可恢复错误的检查先完成。已登记的额度和 nonce 不会因撤销、拒绝消费或结果未知而返还。提交确认丢失注入模拟“已经记账，但确认未送达”。原调用和重试均无法取得成功票据或消费回执，状态查询可看到 `Unconfirmed`。

Clock 由测试注入。墙钟和单调时间都必须严格早于截止点。时钟倒退或不可用使未消费授权失效。policy、grant、core epoch 为独立类型，OS boot ID 独立。观察到 boot 改变即封锁旧 boot，只有未见过的 boot 和新的 core epoch 才能打开新上下文。旧授权和 nonce 历史不可导入为当前许可。

输出类型分别表达假设执行结果、证据等级、合成消费与对账。模块没有进程、IPC、数据库、系统时钟、执行器或 Agent 集成。
