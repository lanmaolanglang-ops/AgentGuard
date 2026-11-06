# I06-D Schema 与绑定边界

| 层 | 复用内容与行为 |
|---|---|
| 原始字节 | 复用 I03 严格 UTF-8／JSON 解析器，拒绝重复或转义等价键、BOM、无效 Unicode、非整数词法及超限。 |
| 操作快照 | 复用 `operation-v2` 本地 Schema。虚拟读取路径额外校验双主体、boot、sandbox 声明、目标索引、对象类型及句柄清单。 |
| 单次票据 | 复用 `permit-v2` 的 22 个字段。完整闭合 Schema，`max_uses = 1`，TTL 1–60000 ms，UTC 日期与两个时钟截止点由同一次样本导出。 |
| 状态迁移 | 复用 `grant-v2` 八状态。内部终止原因独立表示。对 64 组状态对逐一与原 Schema 比较。 |

Worker 实例、连接、Decision、Approval、OS boot、单调截止点和 key generation 放在内部 Reservation，兑换时按完整绑定核对。`runtime_digest` 及两类 principal 摘要在该合成实现中使用 SHA-256(E(对应对象))。句柄清单摘要使用 SHA-256(E(清单))，文件目标的 identity 摘要按 15 使用 SHA-256(E(目标对象))。这些普通摘要与 HMAC 有不同用途。

状态机的正向数据只覆盖一个虚拟 `file.read.v1` 目标及一个不继承的 target 清单条目。该范围来自 OP01 的内存派生副本。冻结样本的占位对象摘要仅用于结构验收，测试在派生副本重算真实的 typed 摘要。虚拟 access 固定为样本中的 `0x00120089`。路径字符串只按字节绑定。完整快照留在活跃 Prepared，退休时释放引用。

内部接口增加显式合成元数据，没有新增线上字段、Schema、RPC 或兼容转换。审查未发现 [07](../../p0.1/07-UNIFIED-EVENT-SCHEMA.md)、[15](../../p0.1/15-EXECUTION-BROKER-CONTRACT.md)、[18](../../p0.1/18-CRYPTOGRAPHIC-CONTRACT.md) 在本次验证字段上的矛盾。非读取 recipe 仅保留冻结 Schema 及密码学向量的覆盖。
