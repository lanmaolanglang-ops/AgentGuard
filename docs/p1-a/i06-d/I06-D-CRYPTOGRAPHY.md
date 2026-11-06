# I06-D 密码学合同

复用 I03 的 E、`AG2` 域帧和 RustCrypto HMAC-SHA-256。验证 MAC 使用 `verify_slice`。标准解析先拒绝重复键和非法数字，E 保留原始 Unicode 与显式 null，按 UTF-8 字节排序键。JSON 序列化仅用于测试票据传输。

| 域 | 输入 |
|---|---|
| secret-v2 | 原始 JSON Pointer、显式值种类和原始字符串。校验排序、重复、重叠、范围与标准 base64 padding，验证后在 shadow 替换。 |
| semantic-v2 | 完整 OperationSnapshot shadow，仅删除 `invocation_id`。 |
| intent-v2 | 语义 MAC、三种 epoch、UTC 期限、`remaining_claims=1`。仅随明确人工批准建立。 |
| operation-v2 | 18 的 14 个字段，绑定语义、invocation、grant、reservation、Broker、Prepared、nonce、key、epoch 与有效期。 |
| permit-v2 | 完整 Permit，仅去掉 `mac`。包含 operation MAC。 |
| audit-v2 | 片段、序号、前序 MAC、脱敏事件的 SHA-256(E(event))、key ID。验证单个链接合同。 |

同一 generation 的六域各用独立的 32 字节合成密钥。secret 值种类通过显式合成元数据传入，冻结的 secret entries 仍只携带 pointer 和 MAC。轮换需要新 key ID、新 generation、新 grant epoch 以及从未在同一模型中使用过的密钥材料。旧未消费票据失效，旧 key ID 的请求直接拒绝。已消费历史保持终态。

Reservation 只保留 SHA-256(raw32 nonce)。原始 nonce 只用于合成预占请求、HMAC 输入和一次出站票据。单进程内持续保留 nonce tombstone。历史 MAC 验证保留策略、生产密钥管理、秘密识别和生产脱敏均有后续门禁。

CV01–CV21 来自冻结文档和 fixture。Rust、Python 和 Node 分别验证编码、帧和 MAC。另用完整合成载荷检查全部六域，独立 Python 构造载荷，Node 复用冻结参考编码器，Rust 对照同一组结果。[冻结合同](../../p0.1/18-CRYPTOGRAPHIC-CONTRACT.md)
