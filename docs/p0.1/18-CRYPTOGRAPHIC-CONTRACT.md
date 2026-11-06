# Typed Encoding、HMAC 与 Nonce 合同

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · AG-06 · 设计合同

## 字节规范

所有整数长度前缀均为无符号big-endian。E(value)递归编码，拒绝无法编码值，禁止自动类型转换。安全数字只接受JSON整数[-9007199254740991,9007199254740991]。原始JSON中的1.0、1e0、NaN、Infinity、-0一律拒绝，布尔值不得被int分支吸收。

| 类型 | 编码 |
|---|---|
| null | 单字节00 |
| false / true | 单字节01 / 02 |
| integer | 03加8字节signed int64 big-endian二补码 |
| string | 04加u32 UTF-8字节长度，再加严格UTF-8字节 |
| 保留类型 | 05保留，接收即拒绝 |
| array | 06加u32元素数量，再按原始顺序连接E(element) |
| map | 07加u32条目数量，key按无符号UTF-8字节词典序排序，再逐项连接E(key)、E(value) |

键必须为字符串。解析前拒绝重复键，排序不赋予重复键合法性。禁止BOM、非法UTF-8和unpaired surrogate。字符串不执行NFC/NFD、大小写、trim或换行转换。UTF-16 Windows字符串先验证完整代理对再转UTF-8。null、空串、空数组和空对象编码不同。

Frame(domain,payload) = ASCII("AG2") || u16be(len(UTF8(domain))) || UTF8(domain) || E(payload)。域字符串固定为小写ASCII，禁止任意调用者选择签名域。MAC = HMAC-SHA-256(K_domain,Frame)。输出64字符小写hex，线上decode后按固定32字节constant-time比较，语法/长度错误先拒绝。正文普通SHA与安全HMAC字段必须明确区分，不能以同长度互换。

编码前最大深度8层，frame最多1MiB，每层对象64字段、数组128项和各Schema更小上限同时生效，边界冲突取更严格限制。参考代码只展示编码核心，生产parser必须另实现这些预算与严格词法拒绝。

## 摘要分层与无环绑定

所有payload使用闭合map，列出的键全部存在，nullable显式编码null。内部Digest合同固定如下。

| 域与输出 | 精确payload |
|---|---|
| secret-v2 / secret_mac | {parameter_pointer, value_kind, value}，kind为utf8或base64_bytes，二进制仅用标准带padding的base64，pointer使用原始JSON Pointer |
| semantic-v2 / semantic_digest | operation-v2对象移除invocation_id，其余全部键保留，包括完整requestor/executor身份和对象identity |
| intent-v2 / intent_digest | {semantic_digest, policy_epoch, grant_epoch, core_boot_epoch, expires_at, remaining_claims}，remaining_claims固定1 |
| operation-v2 / operation_digest | {semantic_digest, invocation_id, grant_id, reservation_id, broker_instance, prepared_id, nonce, key_id, core_boot_epoch, policy_epoch, grant_epoch, expires_at, ttl_ms, max_uses} |
| permit-v2 / permit.mac | 完整permit-v2对象只移除mac键，包含operation_digest |
| audit-v2 / chain_mac | {segment_id, sequence, previous_mac, event_digest, key_id}，event_digest为已脱敏事件E编码的SHA-256 |

先对原始OperationSnapshot执行operation-v2结构验证，再深拷贝为仅供摘要计算的shadow map。secret_parameters.entries按json_pointer的UTF-8字节排序，pointer只允许/command或/arguments下实际存在的非null字符串，禁止重复、祖先/后代重叠及修改身份/摘要等正式安全字段。含秘密的command必须对整个command字符串做secret MAC，不能按字符片段猜测替换。

Core以secret-v2表中payload重新计算每个MAC，匹配后在shadow对应位置替换为闭合map {key_id,secret_mac}，删除invocation_id得到semantic输入。shadow不作为operation-v2传输对象，原始参数仍留在短期执行缓存，实际执行前Core/Broker核对同一数据。所有秘密必须先识别并登记，否则该recipe不得获得敏感参数强绑定认证。不能把UI的同一个“***”当真实输入。

semantic先计算，operation再绑定fresh invocation/nonce/prepared及epoch，permit最后签名。不得把permit.mac、operation_digest或nonce反向加入semantic快照造成循环。域对应独立32字节密钥，不共享密钥，也不从日志摘要推导密钥。

## 路径与语义变化

路径编码保留精确大小写、Unicode、斜杠和前后空格。操作时必须另外验证最终对象、reparse/volume/file identity及访问权限，大小写不同即使指向同一文件也使旧精确授权失效。不能为匹配旧授权自动规范化。

参数、cwd、Agent版本/二进制、adapter、provider、config、SID/LUID/token/进程实例、执行Worker、sandbox策略、文件/脚本/环境/stdin、网络目的/方法/DNS/正文、任一secret MAC和policy/grant/Core epoch变化均使旧permit失效。换invocation必须重新生成operation_digest，不能复用旧ticket。

延后批准意图只有同一已冻结prepared资源/进程身份仍存活、语义完全一致、一次认领时才可能使用。Worker重建导致身份不同必须重新批准。首版默认关闭延后自动认领，等D04确定委托路线并完成AT17/19/30后再评审。模式A无最终兑换点时禁用此机制。

## 密钥与Nonce

| 生命周期项 | 约束 |
|---|---|
| 生成 | Core使用BCryptGenRandom系统CSPRNG分别生成32字节key和32字节nonce，失败拒绝许可 |
| Key ID | 非秘密、唯一、64字符内，ledger记录用途/创建/轮换/停用状态，禁止隐式选“最新”核验旧票 |
| 持有 | 仅Core持有全部HMAC key，Broker在线请求Core Redeem，无签名key副本，UI/Adapter/Worker不接触key |
| 存储 | 候选DPAPI CurrentUser加最小DACL，只提供静态保护，同SID攻击仍可能解密或读内存，D05未决 |
| 轮换 | 策略配置、泄露、部署身份变化或计划周期触发，新key ID、新grant epoch，旧未消费ticket invalidated |
| 审计旧key | 仅按保留规则验证历史段，不能授权新操作，外部锚点/防截尾与回滚范围由D05决定 |
| nonce落库 | 在预占事务生成并登记SHA-256(raw32)，全账本UNIQUE(nonce_hash)，冲突重生成，连续生成异常进入不可授权状态 |
| 消费 | 条件更新reserved与持久审计同一事务，仅一次成功，唯一性检查不能放在事务外 |
| 时间 | monotonic deadline与expires_at双限，墙钟回拨不延长，休眠后重核，Core重启产生新epoch |
| 恢复 | 旧未消费票失效，已消费不退款，不因执行失败变更授权历史，未知执行不可自动重试 |
| 清理 | 过期nonce tombstone至少保存至所有旧epoch不可接受并满足审计保留，不能靠删行恢复可用性 |

Core boot epoch与Windows os_boot_id含义不同。每次Core启动都新建epoch，OS boot ID绑定进程实例。epoch不能凭普通用户可回滚数据库抵抗T2-B。安全主体和防回退信任锚仍待D01/D05决定。

## 跨语言固定向量

下面的向量是编码与域分离测试，简单primitive和binding map不冒充完整OperationSnapshot。CV10–16逐项改变安全绑定字段，CV17/18只改map插入顺序，必须编码相同。CV20/21在secret-v2域验证秘密变化，生产secret结构见上表。公开测试key绝不能用于真实许可。

Python stdlib与Node独立实现本次已对全部21项逐字节核对encoded、frame及MAC。Rust实现尚不存在，I03/I06必须用同向量验证，不能从两语言通过推定Rust通过。

<!-- ag:schema crypto-vectors-v2 -->

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:crypto-vectors:2.0.0",
  "type": "object",
  "properties": {
    "key_id": {
      "const": "PUBLIC_TEST_ONLY"
    },
    "key_hex": {
      "const": "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    },
    "vectors": {
      "type": "array",
      "minItems": 21,
      "maxItems": 21,
      "items": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string",
            "pattern": "^CV[0-9]{2}$"
          },
          "description": {
            "type": "string",
            "maxLength": 128
          },
          "domain": {
            "enum": [
              "operation-v2",
              "secret-v2"
            ]
          },
          "input": {
            "$ref": "#/$defs/value"
          },
          "encoded_hex": {
            "type": "string",
            "pattern": "^[0-9a-f]+$"
          },
          "framed_hex": {
            "type": "string",
            "pattern": "^[0-9a-f]+$"
          },
          "hmac_sha256": {
            "type": "string",
            "pattern": "^[0-9a-f]{64}$"
          }
        },
        "required": [
          "id",
          "description",
          "domain",
          "input",
          "encoded_hex",
          "framed_hex",
          "hmac_sha256"
        ],
        "additionalProperties": false
      }
    }
  },
  "required": [
    "key_id",
    "key_hex",
    "vectors"
  ],
  "additionalProperties": false,
  "$defs": {
    "value": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "type": "boolean"
        },
        {
          "type": "integer",
          "minimum": -9007199254740991,
          "maximum": 9007199254740991
        },
        {
          "type": "string",
          "minLength": 0,
          "maxLength": 16384
        },
        {
          "type": "array",
          "items": {
            "$ref": "#/$defs/value"
          },
          "maxItems": 128,
          "minItems": 0
        },
        {
          "type": "object",
          "maxProperties": 64,
          "propertyNames": {
            "type": "string",
            "minLength": 1,
            "maxLength": 128
          },
          "additionalProperties": {
            "$ref": "#/$defs/value"
          }
        }
      ]
    }
  }
}
~~~

<!-- ag:fixture CV-BUNDLE crypto-vectors-v2 valid -->

~~~json
{
  "key_id": "PUBLIC_TEST_ONLY",
  "key_hex": "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
  "vectors": [
    {
      "id": "CV01",
      "description": "Unicode composed",
      "domain": "operation-v2",
      "input": "é",
      "encoded_hex": "0400000002c3a9",
      "framed_hex": "414732000c6f7065726174696f6e2d76320400000002c3a9",
      "hmac_sha256": "cd58ba76584eae3e36c3834951230bd6ade54efbb62e690c58425a50ec58ecd4"
    },
    {
      "id": "CV02",
      "description": "Unicode decomposed",
      "domain": "operation-v2",
      "input": "é",
      "encoded_hex": "040000000365cc81",
      "framed_hex": "414732000c6f7065726174696f6e2d7632040000000365cc81",
      "hmac_sha256": "89ddef38a4d16a1677a0a617b293ee1bdd58f09301e45d038481c211d0aa9b1d"
    },
    {
      "id": "CV03",
      "description": "Chinese path",
      "domain": "operation-v2",
      "input": "D:\\隔离测试\\资料.txt",
      "encoded_hex": "040000001a443a5ce99a94e7a6bbe6b58be8af955ce8b584e696992e747874",
      "framed_hex": "414732000c6f7065726174696f6e2d7632040000001a443a5ce99a94e7a6bbe6b58be8af955ce8b584e696992e747874",
      "hmac_sha256": "9041c5e4a1cbe29deb78ef56dffefd96ffcb73f4c2a9fffbfbc83b7134c929ae"
    },
    {
      "id": "CV04",
      "description": "Path case upper",
      "domain": "operation-v2",
      "input": "D:\\GuardLab\\Data.txt",
      "encoded_hex": "0400000014443a5c47756172644c61625c446174612e747874",
      "framed_hex": "414732000c6f7065726174696f6e2d76320400000014443a5c47756172644c61625c446174612e747874",
      "hmac_sha256": "2f8980a08ba0b8f7ddcd218407e3ec6839fd3d9fa6d7a3e3b274719a44a704b2"
    },
    {
      "id": "CV05",
      "description": "Path case lower",
      "domain": "operation-v2",
      "input": "d:\\guardlab\\data.txt",
      "encoded_hex": "0400000014643a5c67756172646c61625c646174612e747874",
      "framed_hex": "414732000c6f7065726174696f6e2d76320400000014643a5c67756172646c61625c646174612e747874",
      "hmac_sha256": "bfabf30a3685ccb534e26a28c96ea696d2771b3b39267d88a028185ebad1d9da"
    },
    {
      "id": "CV06",
      "description": "Null",
      "domain": "operation-v2",
      "input": null,
      "encoded_hex": "00",
      "framed_hex": "414732000c6f7065726174696f6e2d763200",
      "hmac_sha256": "fb2882a90a3ab5d2184ca0f9ca064b5f42fc08e82a9ad771711161a2a902c074"
    },
    {
      "id": "CV07",
      "description": "Empty string",
      "domain": "operation-v2",
      "input": "",
      "encoded_hex": "0400000000",
      "framed_hex": "414732000c6f7065726174696f6e2d76320400000000",
      "hmac_sha256": "ced9aae574896b78b280458cde6cb9a8b4a4c4bd6b4bbbf987e20e81c398bf3e"
    },
    {
      "id": "CV08",
      "description": "Array order 1",
      "domain": "operation-v2",
      "input": [
        "read",
        "write"
      ],
      "encoded_hex": "060000000204000000047265616404000000057772697465",
      "framed_hex": "414732000c6f7065726174696f6e2d7632060000000204000000047265616404000000057772697465",
      "hmac_sha256": "9a06836f18ec2cb6f3c514ff55e627b8f3ad260dce36da476ecad29ca15f9986"
    },
    {
      "id": "CV09",
      "description": "Array order 2",
      "domain": "operation-v2",
      "input": [
        "write",
        "read"
      ],
      "encoded_hex": "060000000204000000057772697465040000000472656164",
      "framed_hex": "414732000c6f7065726174696f6e2d7632060000000204000000057772697465040000000472656164",
      "hmac_sha256": "7a521cdfb366aca61c39f034d5dd207020d48cc64f844e08cd94170335d7b1d0"
    },
    {
      "id": "CV10",
      "description": "Binding base",
      "domain": "operation-v2",
      "input": {
        "agent_version": "1.0.0",
        "arguments": {
          "count": 1
        },
        "working_directory": "D:\\GuardLab",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "provider_id": "fixture-provider",
        "secret": {
          "kid": "public-test-secret",
          "mac": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
      },
      "encoded_hex": "0700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "hmac_sha256": "d5119e97cb632198a195dbb8699c95c0235d9b16e0115cf144932b6c848b692a"
    },
    {
      "id": "CV11",
      "description": "Parameter change",
      "domain": "operation-v2",
      "input": {
        "agent_version": "1.0.0",
        "arguments": {
          "count": 2
        },
        "working_directory": "D:\\GuardLab",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "provider_id": "fixture-provider",
        "secret": {
          "kid": "public-test-secret",
          "mac": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
      },
      "encoded_hex": "0700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000002040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000002040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "hmac_sha256": "5f901ede1d63385e67985d00a8ba2445b18b0ea2b031ded135b80bcea8e5efd9"
    },
    {
      "id": "CV12",
      "description": "Cwd change",
      "domain": "operation-v2",
      "input": {
        "agent_version": "1.0.0",
        "arguments": {
          "count": 1
        },
        "working_directory": "D:\\GuardLab\\other",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "provider_id": "fixture-provider",
        "secret": {
          "kid": "public-test-secret",
          "mac": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
      },
      "encoded_hex": "0700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f72790400000011443a5c47756172644c61625c6f74686572",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f72790400000011443a5c47756172644c61625c6f74686572",
      "hmac_sha256": "b2681769583d58d4c89d1117de1569fb94542a76005180865b40fb018c849f20"
    },
    {
      "id": "CV13",
      "description": "Agent version change",
      "domain": "operation-v2",
      "input": {
        "agent_version": "1.0.1",
        "arguments": {
          "count": 1
        },
        "working_directory": "D:\\GuardLab",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "provider_id": "fixture-provider",
        "secret": {
          "kid": "public-test-secret",
          "mac": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
      },
      "encoded_hex": "0700000006040000000d6167656e745f76657273696f6e0400000005312e302e310400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000006040000000d6167656e745f76657273696f6e0400000005312e302e310400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "hmac_sha256": "dacf43ba8faf27839d6a95668188e63d6d15adf25e8faf8893cd239c96449aec"
    },
    {
      "id": "CV14",
      "description": "Token change",
      "domain": "operation-v2",
      "input": {
        "agent_version": "1.0.0",
        "arguments": {
          "count": 1
        },
        "working_directory": "D:\\GuardLab",
        "token_digest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "provider_id": "fixture-provider",
        "secret": {
          "kid": "public-test-secret",
          "mac": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
      },
      "encoded_hex": "0700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363630400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363636363630400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "hmac_sha256": "27b3d7439c7a7b27e9d13e0afa3dfb85ccbddcb2f26229acc984d32c4da8ddfa"
    },
    {
      "id": "CV15",
      "description": "Provider change",
      "domain": "operation-v2",
      "input": {
        "agent_version": "1.0.0",
        "arguments": {
          "count": 1
        },
        "working_directory": "D:\\GuardLab",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "provider_id": "other-provider",
        "secret": {
          "kid": "public-test-secret",
          "mac": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
      },
      "encoded_hex": "0700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f6964040000000e6f746865722d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f6964040000000e6f746865722d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004062626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "hmac_sha256": "7ee854273207cb4ba790196096707d4652bf5005b946cbc69bf9900ed3f25687"
    },
    {
      "id": "CV16",
      "description": "Secret representation change",
      "domain": "operation-v2",
      "input": {
        "agent_version": "1.0.0",
        "arguments": {
          "count": 1
        },
        "working_directory": "D:\\GuardLab",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "provider_id": "fixture-provider",
        "secret": {
          "kid": "public-test-secret",
          "mac": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
        }
      },
      "encoded_hex": "0700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004064646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000006040000000d6167656e745f76657273696f6e0400000005312e302e300400000009617267756d656e747307000000010400000005636f756e74030000000000000001040000000b70726f76696465725f69640400000010666978747572652d70726f76696465720400000006736563726574070000000204000000036b696404000000127075626c69632d746573742d73656372657404000000036d6163040000004064646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464646464040000000c746f6b656e5f6469676573740400000040616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161610400000011776f726b696e675f6469726563746f7279040000000b443a5c47756172644c6162",
      "hmac_sha256": "0b73bf5a7bacb7ec087d4a84286459b863a52a223ae96d69e8548ba61a5bb394"
    },
    {
      "id": "CV17",
      "description": "Map insertion ordering",
      "domain": "operation-v2",
      "input": {
        "b": 2,
        "a": 1
      },
      "encoded_hex": "0700000002040000000161030000000000000001040000000162030000000000000002",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000002040000000161030000000000000001040000000162030000000000000002",
      "hmac_sha256": "ab830695259f7c0f5a1594d2e42bda42ac3e039f2032bc097f5157259f8e4ce1"
    },
    {
      "id": "CV18",
      "description": "Map sorted ordering",
      "domain": "operation-v2",
      "input": {
        "a": 1,
        "b": 2
      },
      "encoded_hex": "0700000002040000000161030000000000000001040000000162030000000000000002",
      "framed_hex": "414732000c6f7065726174696f6e2d76320700000002040000000161030000000000000001040000000162030000000000000002",
      "hmac_sha256": "ab830695259f7c0f5a1594d2e42bda42ac3e039f2032bc097f5157259f8e4ce1"
    },
    {
      "id": "CV19",
      "description": "Integer boundary",
      "domain": "operation-v2",
      "input": 9007199254740991,
      "encoded_hex": "03001fffffffffffff",
      "framed_hex": "414732000c6f7065726174696f6e2d763203001fffffffffffff",
      "hmac_sha256": "c79d77d622bdb58f9d707279a029b62b1aa701f8439c6843c71216d695c382f1"
    },
    {
      "id": "CV20",
      "description": "Secret raw alpha",
      "domain": "secret-v2",
      "input": "fake-secret-alpha",
      "encoded_hex": "040000001166616b652d7365637265742d616c706861",
      "framed_hex": "41473200097365637265742d7632040000001166616b652d7365637265742d616c706861",
      "hmac_sha256": "14016042f45f7e23c1a0c1e002d8b0efaec8ddb770230b9861ce5a550a96dd21"
    },
    {
      "id": "CV21",
      "description": "Secret raw beta",
      "domain": "secret-v2",
      "input": "fake-secret-beta",
      "encoded_hex": "040000001066616b652d7365637265742d62657461",
      "framed_hex": "41473200097365637265742d7632040000001066616b652d7365637265742d62657461",
      "hmac_sha256": "ceab05d0a6404aa9c60514753dda18562d06dff648dff658b1afac1545773d7c"
    }
  ]
}
~~~

## 可执行参考核心

以下代码仅在内存输出公开测试数据，无文件写入、无网络和真实执行。运行验证流程位于20。

<!-- ag:crypto-python -->

~~~python
import json, struct, hmac, hashlib
inputs=json.loads(r'''[["CV01","Unicode composed","é"],["CV02","Unicode decomposed","é"],["CV03","Chinese path","D:\\隔离测试\\资料.txt"],["CV04","Path case upper","D:\\GuardLab\\Data.txt"],["CV05","Path case lower","d:\\guardlab\\data.txt"],["CV06","Null",null],["CV07","Empty string",""],["CV08","Array order 1",["read","write"]],["CV09","Array order 2",["write","read"]],["CV10","Binding base",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV11","Parameter change",{"agent_version":"1.0.0","arguments":{"count":2},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV12","Cwd change",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab\\other","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV13","Agent version change",{"agent_version":"1.0.1","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV14","Token change",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV15","Provider change",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"other-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV16","Secret representation change",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"}}],["CV17","Map insertion ordering",{"b":2,"a":1}],["CV18","Map sorted ordering",{"a":1,"b":2}],["CV19","Integer boundary",9007199254740991],["CV20","Secret raw alpha","fake-secret-alpha"],["CV21","Secret raw beta","fake-secret-beta"]]''')
def encode(value):
    if value is None: return b'\x00'
    if value is False: return b'\x01'
    if value is True: return b'\x02'
    if isinstance(value,int):
        if abs(value)>9007199254740991: raise ValueError('unsafe integer')
        return b'\x03'+struct.pack('>q',value)
    if isinstance(value,str):
        raw=value.encode('utf-8','strict')
        return b'\x04'+struct.pack('>I',len(raw))+raw
    if isinstance(value,list):
        return b'\x06'+struct.pack('>I',len(value))+b''.join(encode(item) for item in value)
    if isinstance(value,dict):
        keys=sorted(value,key=lambda item:item.encode('utf-8'))
        return b'\x07'+struct.pack('>I',len(keys))+b''.join(encode(key)+encode(value[key]) for key in keys)
    raise ValueError('unsupported type')
key=bytes(range(32))
domain=b'operation-v2'
result=[]
for identity,title,value in inputs:
    encoded=encode(value)
    domain=b'secret-v2' if identity in ('CV20','CV21') else b'operation-v2'
    framed=b'AG2'+struct.pack('>H',len(domain))+domain+encoded
    result.append({'id':identity,'description':title,'domain':domain.decode(),'input':value,'encoded_hex':encoded.hex(),'framed_hex':framed.hex(),'hmac_sha256':hmac.new(key,framed,hashlib.sha256).hexdigest()})
print(json.dumps(result,ensure_ascii=True))

~~~

<!-- ag:crypto-node -->

~~~javascript
import {createHmac} from 'node:crypto';
const inputs=[["CV01","Unicode composed","é"],["CV02","Unicode decomposed","é"],["CV03","Chinese path","D:\\隔离测试\\资料.txt"],["CV04","Path case upper","D:\\GuardLab\\Data.txt"],["CV05","Path case lower","d:\\guardlab\\data.txt"],["CV06","Null",null],["CV07","Empty string",""],["CV08","Array order 1",["read","write"]],["CV09","Array order 2",["write","read"]],["CV10","Binding base",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV11","Parameter change",{"agent_version":"1.0.0","arguments":{"count":2},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV12","Cwd change",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab\\other","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV13","Agent version change",{"agent_version":"1.0.1","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV14","Token change",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV15","Provider change",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"other-provider","secret":{"kid":"public-test-secret","mac":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],["CV16","Secret representation change",{"agent_version":"1.0.0","arguments":{"count":1},"working_directory":"D:\\GuardLab","token_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","provider_id":"fixture-provider","secret":{"kid":"public-test-secret","mac":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"}}],["CV17","Map insertion ordering",{"b":2,"a":1}],["CV18","Map sorted ordering",{"a":1,"b":2}],["CV19","Integer boundary",9007199254740991],["CV20","Secret raw alpha","fake-secret-alpha"],["CV21","Secret raw beta","fake-secret-beta"]];
function u32(value){const data=Buffer.alloc(4);data.writeUInt32BE(value);return data;}
function encode(value){
 if(value===null)return Buffer.from([0]);
 if(value===false)return Buffer.from([1]);
 if(value===true)return Buffer.from([2]);
 if(typeof value==='number'){
  if(!Number.isSafeInteger(value))throw new Error('unsafe integer');
  const data=Buffer.alloc(9);data[0]=3;data.writeBigInt64BE(BigInt(value),1);return data;
 }
 if(typeof value==='string'){
  if(!value.isWellFormed())throw new Error('invalid Unicode');
  const data=Buffer.from(value,'utf8');return Buffer.concat([Buffer.from([4]),u32(data.length),data]);
 }
 if(Array.isArray(value))return Buffer.concat([Buffer.from([6]),u32(value.length),...value.map(encode)]);
 const keys=Object.keys(value).sort((a,b)=>Buffer.compare(Buffer.from(a,'utf8'),Buffer.from(b,'utf8')));
 return Buffer.concat([Buffer.from([7]),u32(keys.length),...keys.flatMap(key=>[encode(key),encode(value[key])])]);
}
const key=Buffer.from(Array.from({length:32},(_,index)=>index));

console.log(JSON.stringify(inputs.map(([id,description,input])=>{const domain=Buffer.from(['CV20','CV21'].includes(id)?'secret-v2':'operation-v2','utf8');const length=Buffer.alloc(2);length.writeUInt16BE(domain.length);const encoded=encode(input);const framed=Buffer.concat([Buffer.from('AG2'),length,domain,encoded]);return{id,encoded_hex:encoded.toString('hex'),framed_hex:framed.toString('hex'),hmac_sha256:createHmac('sha256',key).update(framed).digest('hex')};})));

~~~

## 证据与实现门槛

CSPRNG、DPAPI与HMAC参考分别为N13/N14/N15。I05负责持久nonce，I06负责最终绑定，I22负责epoch与unknown恢复。AT17/18/19/22/27/30/34/36/44/45覆盖对应保证。所有真实IPC、密钥保护和Windows运行测试待P1。[来源](13-SOURCE-REGISTER.md) [执行代理](15-EXECUTION-BROKER-CONTRACT.md) [协议验证](20-PROTOCOL-VALIDATION-FIXTURES.md)

D05未决定密钥部署和外部信任锚，不影响合成向量验证，阻止抗同SID/状态回滚的发布承诺。

