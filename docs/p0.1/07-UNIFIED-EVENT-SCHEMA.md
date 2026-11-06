# 统一事件与安全数据协议

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · schema_version 2.0.0 · AG-02 / AG-05 / AG-10

## 目的

冻结事件、身份、网络、授权、操作快照和许可的正式结构。Schema只证明结构，运行可信度、MAC和OS边界由独立核验决定。

## 字段所有权与拒绝规则

Core持有最终事件的received_at、ingest_seq、policy_decision、risk_level、capabilities、grant和digest。Adapter ingress是无权限的提议，Core按角色白名单映射，拒绝Adapter提交最终allow或认证true。本文event-v2描述Core规范化后的事件，不能直接把它当作免校验的Adapter RPC。

principal_snapshot.reported仅表示声明。只有OS核验进程实例、token及关联链后，Core/Broker可填写os_verified。pid不能代替创建时间和OS boot ID。requestor与executor分开绑定，委托执行不能只验证Adapter自身。

所有对象闭合，required列表明确。安全字段不能从extensions读取。extensions最多4个diagnostic命名空间字符串，每个256字符，仅用于诊断。unknown/null表示缺证据，不能作为通配符或空正文。网络观察允许未知，network-grant-v2必须具有完整目标、DNS、正文摘要、原生许可和出口认证范围。

事件的command/arguments/target_paths是脱敏审计表示。operation-v2是短期权威执行快照，含真实typed参数，由Core与Broker在受保护IPC/内存中处理，禁止从脱敏事件重建。原始秘密只在短期执行上下文存在，审计保存secret_parameters的key_id与MAC。日志、错误、dump与UI均需AT34验证。

handle_manifest是Broker实际持有资源的闭合清单，绑定对象类型、identity摘要、access、继承与用途。opaque handle_ref是prepared内部UUID，客户端整数不能作为HANDLE。该清单纳入semantic摘要并在Launch前复核，不允许验证后临时加入额外句柄。

## 固定资源预算

IPC每帧UTF-8最多1MiB，最大嵌套8层，Schema验证前拒绝重复键、非法Unicode、非整数数字和不安全整数。JSON Schema的长度按Unicode字符计算，UTF-8字节预算在parser层核验。数组与字段上限见Schema。拒绝未知major、未知recipe、无权限角色和资源超限，不接受远程Schema下载。

语义层继续检查capability.scope_id与certificate.claim_scope_id相同、主体/版本/OS/证据范围匹配、时间先后、授权期限未到、DNS实际IP在批准集合、IP family一致、target_index有效、file recipe类型一致、nonce唯一及状态转换与真实账本一致。结构合法的伪MAC不构成执行权。

## 生命周期

grant状态唯一采用created、pending、reserved、consumed、revoked、expired、invalidated、failed。created的previous_state为null，pending来自created，reserved来自pending，consumed来自reserved。其余四个终态只可从前三个未消费态进入。consumed不可更改为failed或revoked。工具执行状态独立，失败写execution.failed。[执行合同](15-EXECUTION-BROKER-CONTRACT.md)

## 能力与证据

L0未接入，L1事件观察已验证，L2指定入口的执行前阻止已验证，L3指定范围的Windows OS隔离已验证。五个标志独立，当前七款均false。有效cert需E3，OS标志需E4。L2可有known_fail_open故障限制，UI必须披露。Schema内的合成E3/E4记录仅测试结构。[能力规范](16-PROTECTION-CAPABILITY-CONTRACT.md)

## 正式Schema

types-v2定义共享类型。event-v2内嵌相同defs供独立事件校验，验证器必须检查两份defs完全相同。其余wrapper按本地URN注册，不联网解析引用。Draft固定2020-12，format使用标准FormatChecker，不能静默忽略uuid/date-time/IP格式。

### types-v2

<!-- ag:schema types-v2 -->

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:types:2.0.0",
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
    },
    "process": {
      "type": "object",
      "properties": {
        "pid": {
          "type": "integer",
          "minimum": 1,
          "maximum": 4294967295
        },
        "created_filetime": {
          "type": "string",
          "pattern": "^[0-9]{1,20}$"
        },
        "os_boot_id": {
          "type": "string",
          "format": "uuid"
        }
      },
      "required": [
        "pid",
        "created_filetime",
        "os_boot_id"
      ],
      "additionalProperties": false
    },
    "token": {
      "type": "object",
      "properties": {
        "is_restricted": {
          "type": "boolean"
        },
        "restricting_sids": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 184
          },
          "maxItems": 64,
          "minItems": 0
        },
        "deny_only_sids": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 184
          },
          "maxItems": 64,
          "minItems": 0
        },
        "removed_privileges": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 128
          },
          "maxItems": 64,
          "minItems": 0
        },
        "modified_luid": {
          "type": "string",
          "pattern": "^0x[0-9a-f]{16}$"
        },
        "token_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        }
      },
      "required": [
        "is_restricted",
        "restricting_sids",
        "deny_only_sids",
        "removed_privileges",
        "modified_luid",
        "token_digest"
      ],
      "additionalProperties": false
    },
    "runtime": {
      "type": "object",
      "properties": {
        "product": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "version": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "binary_sha256": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "generation": {
          "type": "string",
          "format": "uuid"
        }
      },
      "required": [
        "product",
        "version",
        "binary_sha256",
        "generation"
      ],
      "additionalProperties": false
    },
    "principal": {
      "type": "object",
      "properties": {
        "windows_sid": {
          "type": "string",
          "pattern": "^S-1-[0-9]+(-[0-9]+){1,15}$",
          "maxLength": 184
        },
        "logon_luid": {
          "type": "string",
          "pattern": "^0x[0-9a-f]{16}$"
        },
        "integrity_level": {
          "enum": [
            "untrusted",
            "low",
            "medium",
            "medium_plus",
            "high",
            "system",
            "protected",
            "unknown"
          ]
        },
        "token_restrictions": {
          "$ref": "#/$defs/token"
        },
        "process_instance": {
          "$ref": "#/$defs/process"
        },
        "parent_process_instance": {
          "anyOf": [
            {
              "$ref": "#/$defs/process"
            },
            {
              "type": "null"
            }
          ]
        },
        "sandbox_identity": {
          "type": "object",
          "properties": {
            "mechanism": {
              "enum": [
                "none",
                "restricted_token",
                "appcontainer",
                "lpac",
                "agent_native",
                "vm",
                "unknown"
              ]
            },
            "identity_id": {
              "anyOf": [
                {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 256
                },
                {
                  "type": "null"
                }
              ]
            },
            "capability_sids": {
              "type": "array",
              "items": {
                "type": "string",
                "minLength": 1,
                "maxLength": 184
              },
              "maxItems": 64,
              "minItems": 0
            },
            "policy_digest": {
              "anyOf": [
                {
                  "type": "string",
                  "pattern": "^[0-9a-f]{64}$"
                },
                {
                  "type": "null"
                }
              ]
            }
          },
          "required": [
            "mechanism",
            "identity_id",
            "capability_sids",
            "policy_digest"
          ],
          "additionalProperties": false
        },
        "agent_runtime_identity": {
          "$ref": "#/$defs/runtime"
        }
      },
      "required": [
        "windows_sid",
        "logon_luid",
        "integrity_level",
        "token_restrictions",
        "process_instance",
        "parent_process_instance",
        "sandbox_identity",
        "agent_runtime_identity"
      ],
      "additionalProperties": false
    },
    "principal_snapshot": {
      "type": "object",
      "properties": {
        "status": {
          "enum": [
            "unknown",
            "claimed",
            "os_verified"
          ]
        },
        "reported": {
          "anyOf": [
            {
              "type": "object",
              "properties": {
                "product": {
                  "anyOf": [
                    {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    {
                      "type": "null"
                    }
                  ]
                },
                "version": {
                  "anyOf": [
                    {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    {
                      "type": "null"
                    }
                  ]
                },
                "pid": {
                  "anyOf": [
                    {
                      "type": "integer",
                      "minimum": 1,
                      "maximum": 4294967295
                    },
                    {
                      "type": "null"
                    }
                  ]
                },
                "sid": {
                  "anyOf": [
                    {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 184
                    },
                    {
                      "type": "null"
                    }
                  ]
                }
              },
              "required": [
                "product",
                "version",
                "pid",
                "sid"
              ],
              "additionalProperties": false
            },
            {
              "type": "null"
            }
          ]
        },
        "os_verified": {
          "anyOf": [
            {
              "$ref": "#/$defs/principal"
            },
            {
              "type": "null"
            }
          ]
        },
        "verifier_instance": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "verified_at": {
          "anyOf": [
            {
              "type": "string",
              "format": "date-time",
              "maxLength": 40
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "status",
        "reported",
        "os_verified",
        "verifier_instance",
        "verified_at"
      ],
      "additionalProperties": false,
      "allOf": [
        {
          "if": {
            "properties": {
              "status": {
                "const": "os_verified"
              }
            }
          },
          "then": {
            "properties": {
              "os_verified": {
                "$ref": "#/$defs/principal"
              },
              "verifier_instance": {
                "type": "string",
                "format": "uuid"
              },
              "verified_at": {
                "type": "string",
                "format": "date-time",
                "maxLength": 40
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "status": {
                "enum": [
                  "unknown",
                  "claimed"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "os_verified": {
                "type": "null"
              },
              "verifier_instance": {
                "type": "null"
              },
              "verified_at": {
                "type": "null"
              }
            }
          }
        }
      ]
    },
    "flagset": {
      "type": "object",
      "properties": {
        "telemetry_verified": {
          "type": "boolean"
        },
        "pre_execution_block_verified": {
          "type": "boolean"
        },
        "parameter_binding_verified": {
          "type": "boolean"
        },
        "single_use_grant_verified": {
          "type": "boolean"
        },
        "os_isolation_verified": {
          "type": "boolean"
        }
      },
      "required": [
        "telemetry_verified",
        "pre_execution_block_verified",
        "parameter_binding_verified",
        "single_use_grant_verified",
        "os_isolation_verified"
      ],
      "additionalProperties": false
    },
    "certificate": {
      "type": "object",
      "properties": {
        "certificate_id": {
          "type": "string",
          "format": "uuid"
        },
        "claim_scope_id": {
          "type": "string",
          "format": "uuid"
        },
        "agent_id": {
          "enum": [
            "A01",
            "A02",
            "A03",
            "A04",
            "A05",
            "A06",
            "A07"
          ]
        },
        "client_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "runtime_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "adapter_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "provider_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "config_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "entrypoint_ids": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 128
          },
          "maxItems": 64,
          "minItems": 1
        },
        "execution_mode": {
          "enum": [
            "A",
            "B",
            "C"
          ]
        },
        "resource_scope": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 512
          },
          "maxItems": 32,
          "minItems": 1
        },
        "flags": {
          "$ref": "#/$defs/flagset"
        },
        "evidence_refs": {
          "type": "array",
          "items": {
            "type": "string",
            "format": "uuid"
          },
          "maxItems": 64,
          "minItems": 1
        },
        "evidence_grade": {
          "enum": [
            "E3",
            "E4"
          ]
        },
        "os_build": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "test_fixture_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "issued_at": {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        "expires_at": {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        "verified_by": {
          "type": "string",
          "format": "uuid"
        },
        "fault_behavior": {
          "enum": [
            "verified_fail_closed",
            "known_fail_open",
            "unknown"
          ]
        },
        "effective_state": {
          "enum": [
            "valid",
            "expired",
            "invalidated",
            "unavailable",
            "unknown"
          ]
        }
      },
      "required": [
        "certificate_id",
        "claim_scope_id",
        "agent_id",
        "client_digest",
        "runtime_digest",
        "adapter_digest",
        "provider_digest",
        "config_digest",
        "entrypoint_ids",
        "execution_mode",
        "resource_scope",
        "flags",
        "evidence_refs",
        "evidence_grade",
        "os_build",
        "test_fixture_digest",
        "issued_at",
        "expires_at",
        "verified_by",
        "fault_behavior",
        "effective_state"
      ],
      "additionalProperties": false
    },
    "capabilities": {
      "type": "object",
      "properties": {
        "scope_id": {
          "type": "string",
          "format": "uuid"
        },
        "level": {
          "enum": [
            "L0",
            "L1",
            "L2",
            "L3"
          ]
        },
        "telemetry_verified": {
          "type": "boolean"
        },
        "pre_execution_block_verified": {
          "type": "boolean"
        },
        "parameter_binding_verified": {
          "type": "boolean"
        },
        "single_use_grant_verified": {
          "type": "boolean"
        },
        "os_isolation_verified": {
          "type": "boolean"
        },
        "certificate": {
          "anyOf": [
            {
              "$ref": "#/$defs/certificate"
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "scope_id",
        "level",
        "telemetry_verified",
        "pre_execution_block_verified",
        "parameter_binding_verified",
        "single_use_grant_verified",
        "os_isolation_verified",
        "certificate"
      ],
      "additionalProperties": false,
      "allOf": [
        {
          "if": {
            "properties": {
              "telemetry_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "telemetry_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "enum": [
                          "E3",
                          "E4"
                        ]
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "telemetry_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "telemetry_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "pre_execution_block_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "pre_execution_block_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "enum": [
                          "E3",
                          "E4"
                        ]
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "pre_execution_block_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "pre_execution_block_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "parameter_binding_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "parameter_binding_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "enum": [
                          "E3",
                          "E4"
                        ]
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "parameter_binding_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "parameter_binding_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "single_use_grant_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "single_use_grant_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "enum": [
                          "E3",
                          "E4"
                        ]
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "single_use_grant_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "single_use_grant_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "os_isolation_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "const": "E4"
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "os_isolation_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "os_isolation_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "level": {
                "const": "L3"
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": false
              },
              "pre_execution_block_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "level": {
                "const": "L2"
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": false
              },
              "pre_execution_block_verified": {
                "const": false
              },
              "telemetry_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "level": {
                "const": "L1"
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": false
              },
              "pre_execution_block_verified": {
                "const": false
              },
              "telemetry_verified": {
                "const": false
              }
            }
          },
          "then": {
            "properties": {
              "level": {
                "const": "L0"
              }
            }
          }
        }
      ]
    },
    "ip": {
      "anyOf": [
        {
          "type": "string",
          "format": "ipv4"
        },
        {
          "type": "string",
          "format": "ipv6"
        }
      ]
    },
    "network": {
      "type": "object",
      "properties": {
        "authorization_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "scope_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "request_method": {
          "enum": [
            null,
            "GET",
            "HEAD",
            "POST",
            "PUT",
            "PATCH",
            "DELETE",
            "OPTIONS",
            "CONNECT",
            "OTHER"
          ]
        },
        "scheme": {
          "enum": [
            null,
            "https",
            "http",
            "other"
          ]
        },
        "host": {
          "anyOf": [
            {
              "type": "string",
              "minLength": 1,
              "maxLength": 253,
              "pattern": "^[a-z0-9][a-z0-9.-]*$"
            },
            {
              "type": "null"
            }
          ]
        },
        "port": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 1,
              "maximum": 65535
            },
            {
              "type": "null"
            }
          ]
        },
        "destination_ip": {
          "anyOf": [
            {
              "$ref": "#/$defs/ip"
            },
            {
              "type": "null"
            }
          ]
        },
        "dns_answers": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "ip": {
                "$ref": "#/$defs/ip"
              },
              "family": {
                "enum": [
                  4,
                  6
                ]
              }
            },
            "required": [
              "ip",
              "family"
            ],
            "additionalProperties": false
          },
          "maxItems": 32,
          "minItems": 0
        },
        "dns_epoch": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "dns_expires_at": {
          "anyOf": [
            {
              "type": "string",
              "format": "date-time",
              "maxLength": 40
            },
            {
              "type": "null"
            }
          ]
        },
        "resolver_id": {
          "anyOf": [
            {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            {
              "type": "null"
            }
          ]
        },
        "redirect_chain": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "method": {
                "type": "string",
                "minLength": 1,
                "maxLength": 16
              },
              "host": {
                "type": "string",
                "minLength": 1,
                "maxLength": 253
              },
              "port": {
                "type": "integer",
                "minimum": 1,
                "maximum": 65535
              },
              "ip": {
                "$ref": "#/$defs/ip"
              },
              "location_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "body_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              }
            },
            "required": [
              "method",
              "host",
              "port",
              "ip",
              "location_digest",
              "body_digest"
            ],
            "additionalProperties": false
          },
          "maxItems": 8,
          "minItems": 0
        },
        "redirect_limit": {
          "type": "integer",
          "minimum": 0,
          "maximum": 8
        },
        "proxy": {
          "type": "object",
          "properties": {
            "mode": {
              "enum": [
                "none",
                "explicit",
                "broker",
                "unknown"
              ]
            },
            "proxy_id": {
              "anyOf": [
                {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                {
                  "type": "null"
                }
              ]
            },
            "endpoint_digest": {
              "anyOf": [
                {
                  "type": "string",
                  "pattern": "^[0-9a-f]{64}$"
                },
                {
                  "type": "null"
                }
              ]
            }
          },
          "required": [
            "mode",
            "proxy_id",
            "endpoint_digest"
          ],
          "additionalProperties": false
        },
        "transport": {
          "enum": [
            "tcp",
            "udp",
            "quic",
            "unknown"
          ]
        },
        "path_query_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "upload_data_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "request_body_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "max_request_bytes": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 0,
              "maximum": 1073741824
            },
            {
              "type": "null"
            }
          ]
        },
        "max_response_bytes": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 0,
              "maximum": 1073741824
            },
            {
              "type": "null"
            }
          ]
        },
        "max_requests": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 1,
              "maximum": 1000
            },
            {
              "type": "null"
            }
          ]
        },
        "connection_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "pool_scope": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "connection_policy_epoch": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 0,
              "maximum": 9007199254740991
            },
            {
              "type": "null"
            }
          ]
        },
        "observation_state": {
          "enum": [
            "claimed",
            "request_seen",
            "connection_seen",
            "sink_verified",
            "unknown"
          ]
        },
        "enforcement_state": {
          "enum": [
            "none",
            "request_proxy",
            "broker_egress",
            "os_verified",
            "unknown"
          ]
        },
        "enforcement_scope_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "native_permission": {
          "enum": [
            "allow",
            "deny",
            "unknown"
          ]
        }
      },
      "required": [
        "authorization_id",
        "scope_id",
        "request_method",
        "scheme",
        "host",
        "port",
        "destination_ip",
        "dns_answers",
        "dns_epoch",
        "dns_expires_at",
        "resolver_id",
        "redirect_chain",
        "redirect_limit",
        "proxy",
        "transport",
        "path_query_digest",
        "upload_data_digest",
        "request_body_digest",
        "max_request_bytes",
        "max_response_bytes",
        "max_requests",
        "connection_id",
        "pool_scope",
        "connection_policy_epoch",
        "observation_state",
        "enforcement_state",
        "enforcement_scope_id",
        "native_permission"
      ],
      "additionalProperties": false
    },
    "network_grant": {
      "type": "object",
      "properties": {
        "authorization_id": {
          "type": "string",
          "format": "uuid"
        },
        "scope_id": {
          "type": "string",
          "format": "uuid"
        },
        "request_method": {
          "enum": [
            "GET",
            "HEAD",
            "POST",
            "PUT",
            "PATCH",
            "DELETE",
            "OPTIONS"
          ]
        },
        "scheme": {
          "enum": [
            "https",
            "http"
          ]
        },
        "host": {
          "type": "string",
          "minLength": 1,
          "maxLength": 253,
          "pattern": "^[a-z0-9][a-z0-9.-]*$"
        },
        "port": {
          "type": "integer",
          "minimum": 1,
          "maximum": 65535
        },
        "destination_ip": {
          "$ref": "#/$defs/ip"
        },
        "dns_answers": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "ip": {
                "$ref": "#/$defs/ip"
              },
              "family": {
                "enum": [
                  4,
                  6
                ]
              }
            },
            "required": [
              "ip",
              "family"
            ],
            "additionalProperties": false
          },
          "maxItems": 32,
          "minItems": 1
        },
        "dns_epoch": {
          "type": "string",
          "format": "uuid"
        },
        "dns_expires_at": {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        "resolver_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "redirect_chain": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "method": {
                "type": "string",
                "minLength": 1,
                "maxLength": 16
              },
              "host": {
                "type": "string",
                "minLength": 1,
                "maxLength": 253
              },
              "port": {
                "type": "integer",
                "minimum": 1,
                "maximum": 65535
              },
              "ip": {
                "$ref": "#/$defs/ip"
              },
              "location_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "body_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              }
            },
            "required": [
              "method",
              "host",
              "port",
              "ip",
              "location_digest",
              "body_digest"
            ],
            "additionalProperties": false
          },
          "maxItems": 8,
          "minItems": 0
        },
        "redirect_limit": {
          "type": "integer",
          "minimum": 0,
          "maximum": 8
        },
        "proxy": {
          "type": "object",
          "properties": {
            "mode": {
              "enum": [
                "none",
                "explicit",
                "broker",
                "unknown"
              ]
            },
            "proxy_id": {
              "anyOf": [
                {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                {
                  "type": "null"
                }
              ]
            },
            "endpoint_digest": {
              "anyOf": [
                {
                  "type": "string",
                  "pattern": "^[0-9a-f]{64}$"
                },
                {
                  "type": "null"
                }
              ]
            }
          },
          "required": [
            "mode",
            "proxy_id",
            "endpoint_digest"
          ],
          "additionalProperties": false
        },
        "transport": {
          "const": "tcp"
        },
        "path_query_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "upload_data_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "request_body_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "max_request_bytes": {
          "type": "integer",
          "minimum": 0,
          "maximum": 16777216
        },
        "max_response_bytes": {
          "type": "integer",
          "minimum": 0,
          "maximum": 16777216
        },
        "max_requests": {
          "const": 1
        },
        "connection_id": {
          "type": "string",
          "format": "uuid"
        },
        "pool_scope": {
          "type": "string",
          "format": "uuid"
        },
        "connection_policy_epoch": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "observation_state": {
          "enum": [
            "claimed",
            "request_seen",
            "connection_seen",
            "sink_verified",
            "unknown"
          ]
        },
        "enforcement_state": {
          "const": "broker_egress"
        },
        "enforcement_scope_id": {
          "type": "string",
          "format": "uuid"
        },
        "native_permission": {
          "const": "allow"
        }
      },
      "required": [
        "authorization_id",
        "scope_id",
        "request_method",
        "scheme",
        "host",
        "port",
        "destination_ip",
        "dns_answers",
        "dns_epoch",
        "dns_expires_at",
        "resolver_id",
        "redirect_chain",
        "redirect_limit",
        "proxy",
        "transport",
        "path_query_digest",
        "upload_data_digest",
        "request_body_digest",
        "max_request_bytes",
        "max_response_bytes",
        "max_requests",
        "connection_id",
        "pool_scope",
        "connection_policy_epoch",
        "observation_state",
        "enforcement_state",
        "enforcement_scope_id",
        "native_permission"
      ],
      "additionalProperties": false
    },
    "object_identity": {
      "type": "object",
      "properties": {
        "path": {
          "type": "string",
          "minLength": 1,
          "maxLength": 32760
        },
        "volume_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "file_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "content_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "kind": {
          "enum": [
            "file",
            "directory"
          ]
        },
        "verification": {
          "const": "handle_verified"
        }
      },
      "required": [
        "path",
        "volume_id",
        "file_id",
        "content_digest",
        "kind",
        "verification"
      ],
      "additionalProperties": false
    },
    "operation": {
      "type": "object",
      "properties": {
        "schema_version": {
          "const": "2.0.0"
        },
        "agent_id": {
          "enum": [
            "A01",
            "A02",
            "A03",
            "A04",
            "A05",
            "A06",
            "A07"
          ]
        },
        "runtime": {
          "$ref": "#/$defs/runtime"
        },
        "adapter_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "adapter_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "provider_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "provider_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "config_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "session_id": {
          "type": "string",
          "format": "uuid"
        },
        "invocation_id": {
          "type": "string",
          "format": "uuid"
        },
        "requestor_principal": {
          "$ref": "#/$defs/principal"
        },
        "executor_principal": {
          "$ref": "#/$defs/principal"
        },
        "recipe": {
          "enum": [
            "file.read.v1",
            "file.write.v1",
            "network.http.v1",
            "process.spawn.experimental.v1"
          ]
        },
        "command": {
          "anyOf": [
            {
              "type": "string",
              "minLength": 1,
              "maxLength": 32760
            },
            {
              "type": "null"
            }
          ]
        },
        "arguments": {
          "$ref": "#/$defs/value"
        },
        "working_directory": {
          "$ref": "#/$defs/object_identity"
        },
        "targets": {
          "type": "array",
          "items": {
            "$ref": "#/$defs/object_identity"
          },
          "maxItems": 64,
          "minItems": 0
        },
        "executable_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "script_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "input_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "environment_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "stdin_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "native_permission": {
          "const": "allow"
        },
        "network_authorization": {
          "anyOf": [
            {
              "$ref": "#/$defs/network_grant"
            },
            {
              "type": "null"
            }
          ]
        },
        "secret_parameters": {
          "type": "object",
          "properties": {
            "key_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 64
            },
            "entries": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "json_pointer": {
                    "type": "string",
                    "pattern": "^/",
                    "maxLength": 512
                  },
                  "secret_mac": {
                    "type": "string",
                    "pattern": "^[0-9a-f]{64}$"
                  }
                },
                "required": [
                  "json_pointer",
                  "secret_mac"
                ],
                "additionalProperties": false
              },
              "maxItems": 32,
              "minItems": 0
            }
          },
          "required": [
            "key_id",
            "entries"
          ],
          "additionalProperties": false
        },
        "handle_manifest": {
          "type": "array",
          "maxItems": 32,
          "items": {
            "type": "object",
            "properties": {
              "handle_ref": {
                "type": "string",
                "format": "uuid"
              },
              "object_type": {
                "enum": [
                  "file",
                  "directory",
                  "pipe",
                  "socket"
                ]
              },
              "object_identity_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "granted_access": {
                "type": "integer",
                "minimum": 0,
                "maximum": 4294967295
              },
              "inherit_to_worker": {
                "type": "boolean"
              },
              "purpose": {
                "enum": [
                  "stdin",
                  "stdout",
                  "stderr",
                  "target",
                  "control"
                ]
              }
            },
            "required": [
              "handle_ref",
              "object_type",
              "object_identity_digest",
              "granted_access",
              "inherit_to_worker",
              "purpose"
            ],
            "additionalProperties": false
          }
        }
      },
      "required": [
        "schema_version",
        "agent_id",
        "runtime",
        "adapter_id",
        "adapter_digest",
        "provider_id",
        "provider_digest",
        "config_digest",
        "session_id",
        "invocation_id",
        "requestor_principal",
        "executor_principal",
        "recipe",
        "command",
        "arguments",
        "working_directory",
        "targets",
        "executable_digest",
        "script_digest",
        "input_digest",
        "environment_digest",
        "stdin_digest",
        "native_permission",
        "network_authorization",
        "secret_parameters",
        "handle_manifest"
      ],
      "additionalProperties": false,
      "allOf": [
        {
          "if": {
            "properties": {
              "recipe": {
                "const": "file.read.v1"
              }
            }
          },
          "then": {
            "properties": {
              "arguments": {
                "type": "object",
                "properties": {
                  "target_index": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 63
                  },
                  "max_bytes": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 16777216
                  }
                },
                "required": [
                  "target_index",
                  "max_bytes"
                ],
                "additionalProperties": false
              },
              "command": {
                "type": "null"
              },
              "network_authorization": {
                "type": "null"
              },
              "targets": {
                "minItems": 1
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "recipe": {
                "const": "file.write.v1"
              }
            }
          },
          "then": {
            "properties": {
              "arguments": {
                "type": "object",
                "properties": {
                  "target_index": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 63
                  },
                  "content_digest": {
                    "type": "string",
                    "pattern": "^[0-9a-f]{64}$"
                  },
                  "max_bytes": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 16777216
                  },
                  "replace_existing": {
                    "type": "boolean"
                  },
                  "leaf_name": {
                    "anyOf": [
                      {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 255,
                        "pattern": "^[^\\\\/:*?\"<>|. ][^\\\\/:*?\"<>|]*[^. ]$|^[a-zA-Z0-9_-]$"
                      },
                      {
                        "type": "null"
                      }
                    ]
                  }
                },
                "required": [
                  "target_index",
                  "content_digest",
                  "max_bytes",
                  "replace_existing",
                  "leaf_name"
                ],
                "additionalProperties": false
              },
              "command": {
                "type": "null"
              },
              "network_authorization": {
                "type": "null"
              },
              "targets": {
                "minItems": 1
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "recipe": {
                "const": "network.http.v1"
              }
            }
          },
          "then": {
            "properties": {
              "arguments": {
                "type": "object",
                "properties": {
                  "request_data_digest": {
                    "type": "string",
                    "pattern": "^[0-9a-f]{64}$"
                  }
                },
                "required": [
                  "request_data_digest"
                ],
                "additionalProperties": false
              },
              "command": {
                "type": "null"
              },
              "network_authorization": {
                "$ref": "#/$defs/network_grant"
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "recipe": {
                "const": "process.spawn.experimental.v1"
              }
            }
          },
          "then": {
            "properties": {
              "arguments": {
                "type": "object",
                "properties": {
                  "argv": {
                    "type": "array",
                    "items": {
                      "type": "string",
                      "minLength": 0,
                      "maxLength": 32760
                    },
                    "maxItems": 128,
                    "minItems": 0
                  },
                  "serializer_id": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 128
                  }
                },
                "required": [
                  "argv",
                  "serializer_id"
                ],
                "additionalProperties": false
              },
              "command": {
                "type": "string",
                "minLength": 1,
                "maxLength": 32760
              },
              "executable_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              }
            }
          }
        }
      ]
    },
    "grant": {
      "type": "object",
      "properties": {
        "grant_id": {
          "type": "string",
          "format": "uuid"
        },
        "state": {
          "enum": [
            "created",
            "pending",
            "reserved",
            "consumed",
            "revoked",
            "expired",
            "invalidated",
            "failed"
          ]
        },
        "previous_state": {
          "enum": [
            null,
            "created",
            "pending",
            "reserved"
          ]
        },
        "reason": {
          "type": "string",
          "minLength": 1,
          "maxLength": 256
        },
        "reservation_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "operation_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "nonce_hash": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "key_id": {
          "anyOf": [
            {
              "type": "string",
              "minLength": 1,
              "maxLength": 64
            },
            {
              "type": "null"
            }
          ]
        },
        "core_boot_epoch": {
          "type": "string",
          "format": "uuid"
        },
        "policy_epoch": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "sequence": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "consumed_by_broker": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "consumed_at": {
          "anyOf": [
            {
              "type": "string",
              "format": "date-time",
              "maxLength": 40
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "grant_id",
        "state",
        "previous_state",
        "reason",
        "reservation_id",
        "operation_digest",
        "nonce_hash",
        "key_id",
        "core_boot_epoch",
        "policy_epoch",
        "sequence",
        "consumed_by_broker",
        "consumed_at"
      ],
      "additionalProperties": false,
      "allOf": [
        {
          "if": {
            "properties": {
              "state": {
                "const": "created"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  null
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "pending"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "reserved"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "pending"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "consumed"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "revoked"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created",
                  "pending",
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "expired"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created",
                  "pending",
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "invalidated"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created",
                  "pending",
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "failed"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created",
                  "pending",
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "enum": [
                  "reserved",
                  "consumed"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "reservation_id": {
                "type": "string",
                "format": "uuid"
              },
              "operation_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "nonce_hash": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "key_id": {
                "type": "string",
                "minLength": 1,
                "maxLength": 64
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "consumed"
              }
            }
          },
          "then": {
            "properties": {
              "consumed_by_broker": {
                "type": "string",
                "format": "uuid"
              },
              "consumed_at": {
                "type": "string",
                "format": "date-time",
                "maxLength": 40
              }
            }
          },
          "else": {
            "properties": {
              "consumed_by_broker": {
                "type": "null"
              },
              "consumed_at": {
                "type": "null"
              }
            }
          }
        }
      ]
    },
    "permit": {
      "type": "object",
      "properties": {
        "schema_version": {
          "const": "2.0.0"
        },
        "grant_id": {
          "type": "string",
          "format": "uuid"
        },
        "reservation_id": {
          "type": "string",
          "format": "uuid"
        },
        "nonce": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "key_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 64
        },
        "core_boot_epoch": {
          "type": "string",
          "format": "uuid"
        },
        "broker_instance": {
          "type": "string",
          "format": "uuid"
        },
        "prepared_id": {
          "type": "string",
          "format": "uuid"
        },
        "invocation_id": {
          "type": "string",
          "format": "uuid"
        },
        "agent_id": {
          "enum": [
            "A01",
            "A02",
            "A03",
            "A04",
            "A05",
            "A06",
            "A07"
          ]
        },
        "session_id": {
          "type": "string",
          "format": "uuid"
        },
        "runtime_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "adapter_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "requestor_principal_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "executor_principal_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "semantic_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "operation_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "policy_epoch": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "grant_epoch": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "expires_at": {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        "ttl_ms": {
          "type": "integer",
          "minimum": 1,
          "maximum": 60000
        },
        "max_uses": {
          "const": 1
        },
        "mac": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        }
      },
      "required": [
        "schema_version",
        "grant_id",
        "reservation_id",
        "nonce",
        "key_id",
        "core_boot_epoch",
        "broker_instance",
        "prepared_id",
        "invocation_id",
        "agent_id",
        "session_id",
        "runtime_digest",
        "adapter_digest",
        "requestor_principal_digest",
        "executor_principal_digest",
        "semantic_digest",
        "operation_digest",
        "policy_epoch",
        "grant_epoch",
        "expires_at",
        "ttl_ms",
        "max_uses",
        "mac"
      ],
      "additionalProperties": false
    }
  }
}
~~~

### event-v2

<!-- ag:schema event-v2 -->

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:event:2.0.0",
  "type": "object",
  "properties": {
    "schema_version": {
      "const": "2.0.0"
    },
    "record_kind": {
      "enum": [
        "runtime",
        "synthetic_fixture",
        "imported"
      ]
    },
    "event_id": {
      "type": "string",
      "format": "uuid"
    },
    "agent_id": {
      "enum": [
        "A01",
        "A02",
        "A03",
        "A04",
        "A05",
        "A06",
        "A07"
      ]
    },
    "agent_version": {
      "anyOf": [
        {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        {
          "type": "null"
        }
      ]
    },
    "session_id": {
      "anyOf": [
        {
          "type": "string",
          "format": "uuid"
        },
        {
          "type": "null"
        }
      ]
    },
    "parent_session_id": {
      "anyOf": [
        {
          "type": "string",
          "format": "uuid"
        },
        {
          "type": "null"
        }
      ]
    },
    "subagent_id": {
      "anyOf": [
        {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        {
          "type": "null"
        }
      ]
    },
    "event_type": {
      "enum": [
        "session.started",
        "session.ended",
        "task.updated",
        "tool.proposed",
        "operation.received",
        "policy.decided",
        "permission.requested",
        "execution.dispatched",
        "execution.started",
        "execution.result",
        "file.observed",
        "network.observed",
        "process.observed",
        "mcp.called",
        "plugin.executed",
        "subagent.started",
        "subagent.ended",
        "agent.crashed",
        "hook.timeout",
        "connection.lost",
        "telemetry.gap",
        "protection.changed",
        "grant.created",
        "grant.pending",
        "grant.reserved",
        "grant.consumed",
        "grant.revoked",
        "grant.expired",
        "grant.invalidated",
        "grant.failed"
      ]
    },
    "timestamp": {
      "anyOf": [
        {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        {
          "type": "null"
        }
      ]
    },
    "received_at": {
      "type": "string",
      "format": "date-time",
      "maxLength": 40
    },
    "correlation_id": {
      "type": "string",
      "format": "uuid"
    },
    "ingest_seq": {
      "type": "integer",
      "minimum": 0,
      "maximum": 9007199254740991
    },
    "tool_name": {
      "anyOf": [
        {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        {
          "type": "null"
        }
      ]
    },
    "command": {
      "anyOf": [
        {
          "type": "string",
          "minLength": 1,
          "maxLength": 32760
        },
        {
          "type": "null"
        }
      ]
    },
    "arguments": {
      "anyOf": [
        {
          "$ref": "#/$defs/value"
        },
        {
          "type": "null"
        }
      ]
    },
    "working_directory": {
      "anyOf": [
        {
          "type": "string",
          "minLength": 1,
          "maxLength": 32760
        },
        {
          "type": "null"
        }
      ]
    },
    "target_paths": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "path": {
            "type": "string",
            "minLength": 1,
            "maxLength": 32760
          },
          "verification": {
            "enum": [
              "claimed",
              "lexical",
              "handle_verified"
            ]
          },
          "object_digest": {
            "anyOf": [
              {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              {
                "type": "null"
              }
            ]
          }
        },
        "required": [
          "path",
          "verification",
          "object_digest"
        ],
        "additionalProperties": false
      },
      "maxItems": 64,
      "minItems": 0
    },
    "network_requests": {
      "type": "array",
      "items": {
        "$ref": "#/$defs/network"
      },
      "maxItems": 16,
      "minItems": 0
    },
    "principal_snapshot": {
      "$ref": "#/$defs/principal_snapshot"
    },
    "execution_mode": {
      "enum": [
        "A",
        "B",
        "C",
        "none"
      ]
    },
    "execution_state": {
      "enum": [
        "proposed",
        "pending",
        "denied",
        "prepared",
        "launch_authorized",
        "running",
        "succeeded",
        "failed",
        "cancelled",
        "unknown"
      ]
    },
    "policy_decision": {
      "enum": [
        null,
        "allow",
        "deny",
        "ask",
        "defer_to_native",
        "not_evaluated"
      ]
    },
    "risk_level": {
      "enum": [
        null,
        "low",
        "medium",
        "high",
        "critical",
        "unknown"
      ]
    },
    "capabilities": {
      "$ref": "#/$defs/capabilities"
    },
    "grant": {
      "anyOf": [
        {
          "$ref": "#/$defs/grant"
        },
        {
          "type": "null"
        }
      ]
    },
    "broker_instance": {
      "anyOf": [
        {
          "type": "string",
          "format": "uuid"
        },
        {
          "type": "null"
        }
      ]
    },
    "prepared_id": {
      "anyOf": [
        {
          "type": "string",
          "format": "uuid"
        },
        {
          "type": "null"
        }
      ]
    },
    "semantic_digest": {
      "anyOf": [
        {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        {
          "type": "null"
        }
      ]
    },
    "operation_digest": {
      "anyOf": [
        {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        {
          "type": "null"
        }
      ]
    },
    "decision_id": {
      "anyOf": [
        {
          "type": "string",
          "format": "uuid"
        },
        {
          "type": "null"
        }
      ]
    },
    "source": {
      "type": "object",
      "properties": {
        "kind": {
          "enum": [
            "adapter",
            "core",
            "broker",
            "observer",
            "import"
          ]
        },
        "instance_id": {
          "type": "string",
          "format": "uuid"
        },
        "sequence": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "invocation_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "authenticated": {
          "type": "boolean"
        },
        "source_event_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        }
      },
      "required": [
        "kind",
        "instance_id",
        "sequence",
        "invocation_id",
        "authenticated",
        "source_event_id"
      ],
      "additionalProperties": false
    },
    "evidence": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "evidence_id": {
            "type": "string",
            "format": "uuid"
          },
          "grade": {
            "enum": [
              "E0",
              "E1",
              "E2",
              "E3",
              "E4"
            ]
          },
          "kind": {
            "enum": [
              "claimed",
              "authenticated_source",
              "independent_effect",
              "os_boundary",
              "unknown"
            ]
          },
          "description": {
            "type": "string",
            "minLength": 1,
            "maxLength": 512
          },
          "artifact_digest": {
            "anyOf": [
              {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              {
                "type": "null"
              }
            ]
          }
        },
        "required": [
          "evidence_id",
          "grade",
          "kind",
          "description",
          "artifact_digest"
        ],
        "additionalProperties": false
      },
      "maxItems": 64,
      "minItems": 0
    },
    "missing_fields": {
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1,
        "maxLength": 128
      },
      "maxItems": 64,
      "minItems": 0
    },
    "redaction": {
      "type": "object",
      "properties": {
        "profile": {
          "const": "audit-safe-v2"
        },
        "fields": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 512
          },
          "maxItems": 128,
          "minItems": 0
        },
        "raw_retained": {
          "const": false
        }
      },
      "required": [
        "profile",
        "fields",
        "raw_retained"
      ],
      "additionalProperties": false
    },
    "extensions": {
      "type": "object",
      "patternProperties": {
        "^diagnostic\\.[a-z0-9_]{1,32}$": {
          "type": "string",
          "minLength": 0,
          "maxLength": 256
        }
      },
      "additionalProperties": false,
      "maxProperties": 4
    },
    "parent_invocation_id": {
      "anyOf": [
        {
          "type": "string",
          "format": "uuid"
        },
        {
          "type": "null"
        }
      ]
    },
    "depth": {
      "type": "integer",
      "minimum": 0,
      "maximum": 32
    },
    "provider_id": {
      "anyOf": [
        {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        {
          "type": "null"
        }
      ]
    }
  },
  "required": [
    "schema_version",
    "record_kind",
    "event_id",
    "agent_id",
    "agent_version",
    "session_id",
    "parent_session_id",
    "subagent_id",
    "event_type",
    "timestamp",
    "received_at",
    "correlation_id",
    "ingest_seq",
    "tool_name",
    "command",
    "arguments",
    "working_directory",
    "target_paths",
    "network_requests",
    "principal_snapshot",
    "execution_mode",
    "execution_state",
    "policy_decision",
    "risk_level",
    "capabilities",
    "grant",
    "broker_instance",
    "prepared_id",
    "semantic_digest",
    "operation_digest",
    "decision_id",
    "source",
    "evidence",
    "missing_fields",
    "redaction",
    "extensions",
    "parent_invocation_id",
    "depth",
    "provider_id"
  ],
  "additionalProperties": false,
  "allOf": [
    {
      "if": {
        "properties": {
          "execution_mode": {
            "const": "A"
          }
        }
      },
      "then": {
        "properties": {
          "policy_decision": {
            "enum": [
              null,
              "deny",
              "ask",
              "defer_to_native",
              "not_evaluated"
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "policy_decision": {
            "const": "allow"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "execution_mode": {
            "const": "B"
          },
          "principal_snapshot": {
            "properties": {
              "status": {
                "const": "os_verified"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "reserved"
                  }
                }
              }
            ]
          },
          "broker_instance": {
            "type": "string",
            "format": "uuid"
          },
          "prepared_id": {
            "type": "string",
            "format": "uuid"
          },
          "semantic_digest": {
            "type": "string",
            "pattern": "^[0-9a-f]{64}$"
          },
          "operation_digest": {
            "type": "string",
            "pattern": "^[0-9a-f]{64}$"
          },
          "decision_id": {
            "type": "string",
            "format": "uuid"
          },
          "event_type": {
            "const": "policy.decided"
          },
          "capabilities": {
            "allOf": [
              {
                "$ref": "#/$defs/capabilities"
              },
              {
                "properties": {
                  "parameter_binding_verified": {
                    "const": true
                  },
                  "single_use_grant_verified": {
                    "const": true
                  }
                }
              }
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "execution_mode": {
            "const": "B"
          },
          "event_type": {
            "const": "execution.started"
          }
        }
      },
      "then": {
        "properties": {
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "consumed"
                  }
                }
              }
            ]
          },
          "principal_snapshot": {
            "properties": {
              "status": {
                "const": "os_verified"
              }
            }
          },
          "broker_instance": {
            "type": "string",
            "format": "uuid"
          },
          "prepared_id": {
            "type": "string",
            "format": "uuid"
          },
          "semantic_digest": {
            "type": "string",
            "pattern": "^[0-9a-f]{64}$"
          },
          "operation_digest": {
            "type": "string",
            "pattern": "^[0-9a-f]{64}$"
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "network.observed"
          }
        }
      },
      "then": {
        "properties": {
          "network_requests": {
            "minItems": 1
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "grant.created"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "created"
                  }
                }
              }
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "grant.pending"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "pending"
                  }
                }
              }
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "grant.reserved"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "reserved"
                  }
                }
              }
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "grant.consumed"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "consumed"
                  }
                }
              }
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "grant.revoked"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "revoked"
                  }
                }
              }
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "grant.expired"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "expired"
                  }
                }
              }
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "grant.invalidated"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "invalidated"
                  }
                }
              }
            ]
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "event_type": {
            "const": "grant.failed"
          }
        }
      },
      "then": {
        "properties": {
          "source": {
            "properties": {
              "kind": {
                "const": "core"
              }
            }
          },
          "grant": {
            "allOf": [
              {
                "$ref": "#/$defs/grant"
              },
              {
                "properties": {
                  "state": {
                    "const": "failed"
                  }
                }
              }
            ]
          }
        }
      }
    }
  ],
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
    },
    "process": {
      "type": "object",
      "properties": {
        "pid": {
          "type": "integer",
          "minimum": 1,
          "maximum": 4294967295
        },
        "created_filetime": {
          "type": "string",
          "pattern": "^[0-9]{1,20}$"
        },
        "os_boot_id": {
          "type": "string",
          "format": "uuid"
        }
      },
      "required": [
        "pid",
        "created_filetime",
        "os_boot_id"
      ],
      "additionalProperties": false
    },
    "token": {
      "type": "object",
      "properties": {
        "is_restricted": {
          "type": "boolean"
        },
        "restricting_sids": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 184
          },
          "maxItems": 64,
          "minItems": 0
        },
        "deny_only_sids": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 184
          },
          "maxItems": 64,
          "minItems": 0
        },
        "removed_privileges": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 128
          },
          "maxItems": 64,
          "minItems": 0
        },
        "modified_luid": {
          "type": "string",
          "pattern": "^0x[0-9a-f]{16}$"
        },
        "token_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        }
      },
      "required": [
        "is_restricted",
        "restricting_sids",
        "deny_only_sids",
        "removed_privileges",
        "modified_luid",
        "token_digest"
      ],
      "additionalProperties": false
    },
    "runtime": {
      "type": "object",
      "properties": {
        "product": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "version": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "binary_sha256": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "generation": {
          "type": "string",
          "format": "uuid"
        }
      },
      "required": [
        "product",
        "version",
        "binary_sha256",
        "generation"
      ],
      "additionalProperties": false
    },
    "principal": {
      "type": "object",
      "properties": {
        "windows_sid": {
          "type": "string",
          "pattern": "^S-1-[0-9]+(-[0-9]+){1,15}$",
          "maxLength": 184
        },
        "logon_luid": {
          "type": "string",
          "pattern": "^0x[0-9a-f]{16}$"
        },
        "integrity_level": {
          "enum": [
            "untrusted",
            "low",
            "medium",
            "medium_plus",
            "high",
            "system",
            "protected",
            "unknown"
          ]
        },
        "token_restrictions": {
          "$ref": "#/$defs/token"
        },
        "process_instance": {
          "$ref": "#/$defs/process"
        },
        "parent_process_instance": {
          "anyOf": [
            {
              "$ref": "#/$defs/process"
            },
            {
              "type": "null"
            }
          ]
        },
        "sandbox_identity": {
          "type": "object",
          "properties": {
            "mechanism": {
              "enum": [
                "none",
                "restricted_token",
                "appcontainer",
                "lpac",
                "agent_native",
                "vm",
                "unknown"
              ]
            },
            "identity_id": {
              "anyOf": [
                {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 256
                },
                {
                  "type": "null"
                }
              ]
            },
            "capability_sids": {
              "type": "array",
              "items": {
                "type": "string",
                "minLength": 1,
                "maxLength": 184
              },
              "maxItems": 64,
              "minItems": 0
            },
            "policy_digest": {
              "anyOf": [
                {
                  "type": "string",
                  "pattern": "^[0-9a-f]{64}$"
                },
                {
                  "type": "null"
                }
              ]
            }
          },
          "required": [
            "mechanism",
            "identity_id",
            "capability_sids",
            "policy_digest"
          ],
          "additionalProperties": false
        },
        "agent_runtime_identity": {
          "$ref": "#/$defs/runtime"
        }
      },
      "required": [
        "windows_sid",
        "logon_luid",
        "integrity_level",
        "token_restrictions",
        "process_instance",
        "parent_process_instance",
        "sandbox_identity",
        "agent_runtime_identity"
      ],
      "additionalProperties": false
    },
    "principal_snapshot": {
      "type": "object",
      "properties": {
        "status": {
          "enum": [
            "unknown",
            "claimed",
            "os_verified"
          ]
        },
        "reported": {
          "anyOf": [
            {
              "type": "object",
              "properties": {
                "product": {
                  "anyOf": [
                    {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    {
                      "type": "null"
                    }
                  ]
                },
                "version": {
                  "anyOf": [
                    {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 128
                    },
                    {
                      "type": "null"
                    }
                  ]
                },
                "pid": {
                  "anyOf": [
                    {
                      "type": "integer",
                      "minimum": 1,
                      "maximum": 4294967295
                    },
                    {
                      "type": "null"
                    }
                  ]
                },
                "sid": {
                  "anyOf": [
                    {
                      "type": "string",
                      "minLength": 1,
                      "maxLength": 184
                    },
                    {
                      "type": "null"
                    }
                  ]
                }
              },
              "required": [
                "product",
                "version",
                "pid",
                "sid"
              ],
              "additionalProperties": false
            },
            {
              "type": "null"
            }
          ]
        },
        "os_verified": {
          "anyOf": [
            {
              "$ref": "#/$defs/principal"
            },
            {
              "type": "null"
            }
          ]
        },
        "verifier_instance": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "verified_at": {
          "anyOf": [
            {
              "type": "string",
              "format": "date-time",
              "maxLength": 40
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "status",
        "reported",
        "os_verified",
        "verifier_instance",
        "verified_at"
      ],
      "additionalProperties": false,
      "allOf": [
        {
          "if": {
            "properties": {
              "status": {
                "const": "os_verified"
              }
            }
          },
          "then": {
            "properties": {
              "os_verified": {
                "$ref": "#/$defs/principal"
              },
              "verifier_instance": {
                "type": "string",
                "format": "uuid"
              },
              "verified_at": {
                "type": "string",
                "format": "date-time",
                "maxLength": 40
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "status": {
                "enum": [
                  "unknown",
                  "claimed"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "os_verified": {
                "type": "null"
              },
              "verifier_instance": {
                "type": "null"
              },
              "verified_at": {
                "type": "null"
              }
            }
          }
        }
      ]
    },
    "flagset": {
      "type": "object",
      "properties": {
        "telemetry_verified": {
          "type": "boolean"
        },
        "pre_execution_block_verified": {
          "type": "boolean"
        },
        "parameter_binding_verified": {
          "type": "boolean"
        },
        "single_use_grant_verified": {
          "type": "boolean"
        },
        "os_isolation_verified": {
          "type": "boolean"
        }
      },
      "required": [
        "telemetry_verified",
        "pre_execution_block_verified",
        "parameter_binding_verified",
        "single_use_grant_verified",
        "os_isolation_verified"
      ],
      "additionalProperties": false
    },
    "certificate": {
      "type": "object",
      "properties": {
        "certificate_id": {
          "type": "string",
          "format": "uuid"
        },
        "claim_scope_id": {
          "type": "string",
          "format": "uuid"
        },
        "agent_id": {
          "enum": [
            "A01",
            "A02",
            "A03",
            "A04",
            "A05",
            "A06",
            "A07"
          ]
        },
        "client_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "runtime_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "adapter_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "provider_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "config_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "entrypoint_ids": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 128
          },
          "maxItems": 64,
          "minItems": 1
        },
        "execution_mode": {
          "enum": [
            "A",
            "B",
            "C"
          ]
        },
        "resource_scope": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": 512
          },
          "maxItems": 32,
          "minItems": 1
        },
        "flags": {
          "$ref": "#/$defs/flagset"
        },
        "evidence_refs": {
          "type": "array",
          "items": {
            "type": "string",
            "format": "uuid"
          },
          "maxItems": 64,
          "minItems": 1
        },
        "evidence_grade": {
          "enum": [
            "E3",
            "E4"
          ]
        },
        "os_build": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "test_fixture_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "issued_at": {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        "expires_at": {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        "verified_by": {
          "type": "string",
          "format": "uuid"
        },
        "fault_behavior": {
          "enum": [
            "verified_fail_closed",
            "known_fail_open",
            "unknown"
          ]
        },
        "effective_state": {
          "enum": [
            "valid",
            "expired",
            "invalidated",
            "unavailable",
            "unknown"
          ]
        }
      },
      "required": [
        "certificate_id",
        "claim_scope_id",
        "agent_id",
        "client_digest",
        "runtime_digest",
        "adapter_digest",
        "provider_digest",
        "config_digest",
        "entrypoint_ids",
        "execution_mode",
        "resource_scope",
        "flags",
        "evidence_refs",
        "evidence_grade",
        "os_build",
        "test_fixture_digest",
        "issued_at",
        "expires_at",
        "verified_by",
        "fault_behavior",
        "effective_state"
      ],
      "additionalProperties": false
    },
    "capabilities": {
      "type": "object",
      "properties": {
        "scope_id": {
          "type": "string",
          "format": "uuid"
        },
        "level": {
          "enum": [
            "L0",
            "L1",
            "L2",
            "L3"
          ]
        },
        "telemetry_verified": {
          "type": "boolean"
        },
        "pre_execution_block_verified": {
          "type": "boolean"
        },
        "parameter_binding_verified": {
          "type": "boolean"
        },
        "single_use_grant_verified": {
          "type": "boolean"
        },
        "os_isolation_verified": {
          "type": "boolean"
        },
        "certificate": {
          "anyOf": [
            {
              "$ref": "#/$defs/certificate"
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "scope_id",
        "level",
        "telemetry_verified",
        "pre_execution_block_verified",
        "parameter_binding_verified",
        "single_use_grant_verified",
        "os_isolation_verified",
        "certificate"
      ],
      "additionalProperties": false,
      "allOf": [
        {
          "if": {
            "properties": {
              "telemetry_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "telemetry_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "enum": [
                          "E3",
                          "E4"
                        ]
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "telemetry_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "telemetry_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "pre_execution_block_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "pre_execution_block_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "enum": [
                          "E3",
                          "E4"
                        ]
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "pre_execution_block_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "pre_execution_block_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "parameter_binding_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "parameter_binding_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "enum": [
                          "E3",
                          "E4"
                        ]
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "parameter_binding_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "parameter_binding_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "single_use_grant_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "single_use_grant_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "enum": [
                          "E3",
                          "E4"
                        ]
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "single_use_grant_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "single_use_grant_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "certificate": {
                "allOf": [
                  {
                    "$ref": "#/$defs/certificate"
                  },
                  {
                    "properties": {
                      "effective_state": {
                        "const": "valid"
                      },
                      "flags": {
                        "properties": {
                          "os_isolation_verified": {
                            "const": true
                          }
                        }
                      },
                      "evidence_grade": {
                        "const": "E4"
                      }
                    }
                  }
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "certificate": {
                "type": "object",
                "properties": {
                  "flags": {
                    "properties": {
                      "os_isolation_verified": {
                        "const": true
                      }
                    }
                  },
                  "effective_state": {
                    "const": "valid"
                  }
                },
                "required": [
                  "flags",
                  "effective_state"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "os_isolation_verified": {
                "const": true
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "level": {
                "const": "L3"
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": false
              },
              "pre_execution_block_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "level": {
                "const": "L2"
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": false
              },
              "pre_execution_block_verified": {
                "const": false
              },
              "telemetry_verified": {
                "const": true
              }
            }
          },
          "then": {
            "properties": {
              "level": {
                "const": "L1"
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "os_isolation_verified": {
                "const": false
              },
              "pre_execution_block_verified": {
                "const": false
              },
              "telemetry_verified": {
                "const": false
              }
            }
          },
          "then": {
            "properties": {
              "level": {
                "const": "L0"
              }
            }
          }
        }
      ]
    },
    "ip": {
      "anyOf": [
        {
          "type": "string",
          "format": "ipv4"
        },
        {
          "type": "string",
          "format": "ipv6"
        }
      ]
    },
    "network": {
      "type": "object",
      "properties": {
        "authorization_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "scope_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "request_method": {
          "enum": [
            null,
            "GET",
            "HEAD",
            "POST",
            "PUT",
            "PATCH",
            "DELETE",
            "OPTIONS",
            "CONNECT",
            "OTHER"
          ]
        },
        "scheme": {
          "enum": [
            null,
            "https",
            "http",
            "other"
          ]
        },
        "host": {
          "anyOf": [
            {
              "type": "string",
              "minLength": 1,
              "maxLength": 253,
              "pattern": "^[a-z0-9][a-z0-9.-]*$"
            },
            {
              "type": "null"
            }
          ]
        },
        "port": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 1,
              "maximum": 65535
            },
            {
              "type": "null"
            }
          ]
        },
        "destination_ip": {
          "anyOf": [
            {
              "$ref": "#/$defs/ip"
            },
            {
              "type": "null"
            }
          ]
        },
        "dns_answers": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "ip": {
                "$ref": "#/$defs/ip"
              },
              "family": {
                "enum": [
                  4,
                  6
                ]
              }
            },
            "required": [
              "ip",
              "family"
            ],
            "additionalProperties": false
          },
          "maxItems": 32,
          "minItems": 0
        },
        "dns_epoch": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "dns_expires_at": {
          "anyOf": [
            {
              "type": "string",
              "format": "date-time",
              "maxLength": 40
            },
            {
              "type": "null"
            }
          ]
        },
        "resolver_id": {
          "anyOf": [
            {
              "type": "string",
              "minLength": 1,
              "maxLength": 128
            },
            {
              "type": "null"
            }
          ]
        },
        "redirect_chain": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "method": {
                "type": "string",
                "minLength": 1,
                "maxLength": 16
              },
              "host": {
                "type": "string",
                "minLength": 1,
                "maxLength": 253
              },
              "port": {
                "type": "integer",
                "minimum": 1,
                "maximum": 65535
              },
              "ip": {
                "$ref": "#/$defs/ip"
              },
              "location_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "body_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              }
            },
            "required": [
              "method",
              "host",
              "port",
              "ip",
              "location_digest",
              "body_digest"
            ],
            "additionalProperties": false
          },
          "maxItems": 8,
          "minItems": 0
        },
        "redirect_limit": {
          "type": "integer",
          "minimum": 0,
          "maximum": 8
        },
        "proxy": {
          "type": "object",
          "properties": {
            "mode": {
              "enum": [
                "none",
                "explicit",
                "broker",
                "unknown"
              ]
            },
            "proxy_id": {
              "anyOf": [
                {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                {
                  "type": "null"
                }
              ]
            },
            "endpoint_digest": {
              "anyOf": [
                {
                  "type": "string",
                  "pattern": "^[0-9a-f]{64}$"
                },
                {
                  "type": "null"
                }
              ]
            }
          },
          "required": [
            "mode",
            "proxy_id",
            "endpoint_digest"
          ],
          "additionalProperties": false
        },
        "transport": {
          "enum": [
            "tcp",
            "udp",
            "quic",
            "unknown"
          ]
        },
        "path_query_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "upload_data_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "request_body_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "max_request_bytes": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 0,
              "maximum": 1073741824
            },
            {
              "type": "null"
            }
          ]
        },
        "max_response_bytes": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 0,
              "maximum": 1073741824
            },
            {
              "type": "null"
            }
          ]
        },
        "max_requests": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 1,
              "maximum": 1000
            },
            {
              "type": "null"
            }
          ]
        },
        "connection_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "pool_scope": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "connection_policy_epoch": {
          "anyOf": [
            {
              "type": "integer",
              "minimum": 0,
              "maximum": 9007199254740991
            },
            {
              "type": "null"
            }
          ]
        },
        "observation_state": {
          "enum": [
            "claimed",
            "request_seen",
            "connection_seen",
            "sink_verified",
            "unknown"
          ]
        },
        "enforcement_state": {
          "enum": [
            "none",
            "request_proxy",
            "broker_egress",
            "os_verified",
            "unknown"
          ]
        },
        "enforcement_scope_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "native_permission": {
          "enum": [
            "allow",
            "deny",
            "unknown"
          ]
        }
      },
      "required": [
        "authorization_id",
        "scope_id",
        "request_method",
        "scheme",
        "host",
        "port",
        "destination_ip",
        "dns_answers",
        "dns_epoch",
        "dns_expires_at",
        "resolver_id",
        "redirect_chain",
        "redirect_limit",
        "proxy",
        "transport",
        "path_query_digest",
        "upload_data_digest",
        "request_body_digest",
        "max_request_bytes",
        "max_response_bytes",
        "max_requests",
        "connection_id",
        "pool_scope",
        "connection_policy_epoch",
        "observation_state",
        "enforcement_state",
        "enforcement_scope_id",
        "native_permission"
      ],
      "additionalProperties": false
    },
    "network_grant": {
      "type": "object",
      "properties": {
        "authorization_id": {
          "type": "string",
          "format": "uuid"
        },
        "scope_id": {
          "type": "string",
          "format": "uuid"
        },
        "request_method": {
          "enum": [
            "GET",
            "HEAD",
            "POST",
            "PUT",
            "PATCH",
            "DELETE",
            "OPTIONS"
          ]
        },
        "scheme": {
          "enum": [
            "https",
            "http"
          ]
        },
        "host": {
          "type": "string",
          "minLength": 1,
          "maxLength": 253,
          "pattern": "^[a-z0-9][a-z0-9.-]*$"
        },
        "port": {
          "type": "integer",
          "minimum": 1,
          "maximum": 65535
        },
        "destination_ip": {
          "$ref": "#/$defs/ip"
        },
        "dns_answers": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "ip": {
                "$ref": "#/$defs/ip"
              },
              "family": {
                "enum": [
                  4,
                  6
                ]
              }
            },
            "required": [
              "ip",
              "family"
            ],
            "additionalProperties": false
          },
          "maxItems": 32,
          "minItems": 1
        },
        "dns_epoch": {
          "type": "string",
          "format": "uuid"
        },
        "dns_expires_at": {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        "resolver_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "redirect_chain": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "method": {
                "type": "string",
                "minLength": 1,
                "maxLength": 16
              },
              "host": {
                "type": "string",
                "minLength": 1,
                "maxLength": 253
              },
              "port": {
                "type": "integer",
                "minimum": 1,
                "maximum": 65535
              },
              "ip": {
                "$ref": "#/$defs/ip"
              },
              "location_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "body_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              }
            },
            "required": [
              "method",
              "host",
              "port",
              "ip",
              "location_digest",
              "body_digest"
            ],
            "additionalProperties": false
          },
          "maxItems": 8,
          "minItems": 0
        },
        "redirect_limit": {
          "type": "integer",
          "minimum": 0,
          "maximum": 8
        },
        "proxy": {
          "type": "object",
          "properties": {
            "mode": {
              "enum": [
                "none",
                "explicit",
                "broker",
                "unknown"
              ]
            },
            "proxy_id": {
              "anyOf": [
                {
                  "type": "string",
                  "minLength": 1,
                  "maxLength": 128
                },
                {
                  "type": "null"
                }
              ]
            },
            "endpoint_digest": {
              "anyOf": [
                {
                  "type": "string",
                  "pattern": "^[0-9a-f]{64}$"
                },
                {
                  "type": "null"
                }
              ]
            }
          },
          "required": [
            "mode",
            "proxy_id",
            "endpoint_digest"
          ],
          "additionalProperties": false
        },
        "transport": {
          "const": "tcp"
        },
        "path_query_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "upload_data_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "request_body_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "max_request_bytes": {
          "type": "integer",
          "minimum": 0,
          "maximum": 16777216
        },
        "max_response_bytes": {
          "type": "integer",
          "minimum": 0,
          "maximum": 16777216
        },
        "max_requests": {
          "const": 1
        },
        "connection_id": {
          "type": "string",
          "format": "uuid"
        },
        "pool_scope": {
          "type": "string",
          "format": "uuid"
        },
        "connection_policy_epoch": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "observation_state": {
          "enum": [
            "claimed",
            "request_seen",
            "connection_seen",
            "sink_verified",
            "unknown"
          ]
        },
        "enforcement_state": {
          "const": "broker_egress"
        },
        "enforcement_scope_id": {
          "type": "string",
          "format": "uuid"
        },
        "native_permission": {
          "const": "allow"
        }
      },
      "required": [
        "authorization_id",
        "scope_id",
        "request_method",
        "scheme",
        "host",
        "port",
        "destination_ip",
        "dns_answers",
        "dns_epoch",
        "dns_expires_at",
        "resolver_id",
        "redirect_chain",
        "redirect_limit",
        "proxy",
        "transport",
        "path_query_digest",
        "upload_data_digest",
        "request_body_digest",
        "max_request_bytes",
        "max_response_bytes",
        "max_requests",
        "connection_id",
        "pool_scope",
        "connection_policy_epoch",
        "observation_state",
        "enforcement_state",
        "enforcement_scope_id",
        "native_permission"
      ],
      "additionalProperties": false
    },
    "object_identity": {
      "type": "object",
      "properties": {
        "path": {
          "type": "string",
          "minLength": 1,
          "maxLength": 32760
        },
        "volume_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "file_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "content_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "kind": {
          "enum": [
            "file",
            "directory"
          ]
        },
        "verification": {
          "const": "handle_verified"
        }
      },
      "required": [
        "path",
        "volume_id",
        "file_id",
        "content_digest",
        "kind",
        "verification"
      ],
      "additionalProperties": false
    },
    "operation": {
      "type": "object",
      "properties": {
        "schema_version": {
          "const": "2.0.0"
        },
        "agent_id": {
          "enum": [
            "A01",
            "A02",
            "A03",
            "A04",
            "A05",
            "A06",
            "A07"
          ]
        },
        "runtime": {
          "$ref": "#/$defs/runtime"
        },
        "adapter_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "adapter_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "provider_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 128
        },
        "provider_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "config_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "session_id": {
          "type": "string",
          "format": "uuid"
        },
        "invocation_id": {
          "type": "string",
          "format": "uuid"
        },
        "requestor_principal": {
          "$ref": "#/$defs/principal"
        },
        "executor_principal": {
          "$ref": "#/$defs/principal"
        },
        "recipe": {
          "enum": [
            "file.read.v1",
            "file.write.v1",
            "network.http.v1",
            "process.spawn.experimental.v1"
          ]
        },
        "command": {
          "anyOf": [
            {
              "type": "string",
              "minLength": 1,
              "maxLength": 32760
            },
            {
              "type": "null"
            }
          ]
        },
        "arguments": {
          "$ref": "#/$defs/value"
        },
        "working_directory": {
          "$ref": "#/$defs/object_identity"
        },
        "targets": {
          "type": "array",
          "items": {
            "$ref": "#/$defs/object_identity"
          },
          "maxItems": 64,
          "minItems": 0
        },
        "executable_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "script_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "input_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "environment_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "stdin_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "native_permission": {
          "const": "allow"
        },
        "network_authorization": {
          "anyOf": [
            {
              "$ref": "#/$defs/network_grant"
            },
            {
              "type": "null"
            }
          ]
        },
        "secret_parameters": {
          "type": "object",
          "properties": {
            "key_id": {
              "type": "string",
              "minLength": 1,
              "maxLength": 64
            },
            "entries": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "json_pointer": {
                    "type": "string",
                    "pattern": "^/",
                    "maxLength": 512
                  },
                  "secret_mac": {
                    "type": "string",
                    "pattern": "^[0-9a-f]{64}$"
                  }
                },
                "required": [
                  "json_pointer",
                  "secret_mac"
                ],
                "additionalProperties": false
              },
              "maxItems": 32,
              "minItems": 0
            }
          },
          "required": [
            "key_id",
            "entries"
          ],
          "additionalProperties": false
        },
        "handle_manifest": {
          "type": "array",
          "maxItems": 32,
          "items": {
            "type": "object",
            "properties": {
              "handle_ref": {
                "type": "string",
                "format": "uuid"
              },
              "object_type": {
                "enum": [
                  "file",
                  "directory",
                  "pipe",
                  "socket"
                ]
              },
              "object_identity_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "granted_access": {
                "type": "integer",
                "minimum": 0,
                "maximum": 4294967295
              },
              "inherit_to_worker": {
                "type": "boolean"
              },
              "purpose": {
                "enum": [
                  "stdin",
                  "stdout",
                  "stderr",
                  "target",
                  "control"
                ]
              }
            },
            "required": [
              "handle_ref",
              "object_type",
              "object_identity_digest",
              "granted_access",
              "inherit_to_worker",
              "purpose"
            ],
            "additionalProperties": false
          }
        }
      },
      "required": [
        "schema_version",
        "agent_id",
        "runtime",
        "adapter_id",
        "adapter_digest",
        "provider_id",
        "provider_digest",
        "config_digest",
        "session_id",
        "invocation_id",
        "requestor_principal",
        "executor_principal",
        "recipe",
        "command",
        "arguments",
        "working_directory",
        "targets",
        "executable_digest",
        "script_digest",
        "input_digest",
        "environment_digest",
        "stdin_digest",
        "native_permission",
        "network_authorization",
        "secret_parameters",
        "handle_manifest"
      ],
      "additionalProperties": false,
      "allOf": [
        {
          "if": {
            "properties": {
              "recipe": {
                "const": "file.read.v1"
              }
            }
          },
          "then": {
            "properties": {
              "arguments": {
                "type": "object",
                "properties": {
                  "target_index": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 63
                  },
                  "max_bytes": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 16777216
                  }
                },
                "required": [
                  "target_index",
                  "max_bytes"
                ],
                "additionalProperties": false
              },
              "command": {
                "type": "null"
              },
              "network_authorization": {
                "type": "null"
              },
              "targets": {
                "minItems": 1
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "recipe": {
                "const": "file.write.v1"
              }
            }
          },
          "then": {
            "properties": {
              "arguments": {
                "type": "object",
                "properties": {
                  "target_index": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 63
                  },
                  "content_digest": {
                    "type": "string",
                    "pattern": "^[0-9a-f]{64}$"
                  },
                  "max_bytes": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 16777216
                  },
                  "replace_existing": {
                    "type": "boolean"
                  },
                  "leaf_name": {
                    "anyOf": [
                      {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 255,
                        "pattern": "^[^\\\\/:*?\"<>|. ][^\\\\/:*?\"<>|]*[^. ]$|^[a-zA-Z0-9_-]$"
                      },
                      {
                        "type": "null"
                      }
                    ]
                  }
                },
                "required": [
                  "target_index",
                  "content_digest",
                  "max_bytes",
                  "replace_existing",
                  "leaf_name"
                ],
                "additionalProperties": false
              },
              "command": {
                "type": "null"
              },
              "network_authorization": {
                "type": "null"
              },
              "targets": {
                "minItems": 1
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "recipe": {
                "const": "network.http.v1"
              }
            }
          },
          "then": {
            "properties": {
              "arguments": {
                "type": "object",
                "properties": {
                  "request_data_digest": {
                    "type": "string",
                    "pattern": "^[0-9a-f]{64}$"
                  }
                },
                "required": [
                  "request_data_digest"
                ],
                "additionalProperties": false
              },
              "command": {
                "type": "null"
              },
              "network_authorization": {
                "$ref": "#/$defs/network_grant"
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "recipe": {
                "const": "process.spawn.experimental.v1"
              }
            }
          },
          "then": {
            "properties": {
              "arguments": {
                "type": "object",
                "properties": {
                  "argv": {
                    "type": "array",
                    "items": {
                      "type": "string",
                      "minLength": 0,
                      "maxLength": 32760
                    },
                    "maxItems": 128,
                    "minItems": 0
                  },
                  "serializer_id": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 128
                  }
                },
                "required": [
                  "argv",
                  "serializer_id"
                ],
                "additionalProperties": false
              },
              "command": {
                "type": "string",
                "minLength": 1,
                "maxLength": 32760
              },
              "executable_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              }
            }
          }
        }
      ]
    },
    "grant": {
      "type": "object",
      "properties": {
        "grant_id": {
          "type": "string",
          "format": "uuid"
        },
        "state": {
          "enum": [
            "created",
            "pending",
            "reserved",
            "consumed",
            "revoked",
            "expired",
            "invalidated",
            "failed"
          ]
        },
        "previous_state": {
          "enum": [
            null,
            "created",
            "pending",
            "reserved"
          ]
        },
        "reason": {
          "type": "string",
          "minLength": 1,
          "maxLength": 256
        },
        "reservation_id": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "operation_digest": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "nonce_hash": {
          "anyOf": [
            {
              "type": "string",
              "pattern": "^[0-9a-f]{64}$"
            },
            {
              "type": "null"
            }
          ]
        },
        "key_id": {
          "anyOf": [
            {
              "type": "string",
              "minLength": 1,
              "maxLength": 64
            },
            {
              "type": "null"
            }
          ]
        },
        "core_boot_epoch": {
          "type": "string",
          "format": "uuid"
        },
        "policy_epoch": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "sequence": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "consumed_by_broker": {
          "anyOf": [
            {
              "type": "string",
              "format": "uuid"
            },
            {
              "type": "null"
            }
          ]
        },
        "consumed_at": {
          "anyOf": [
            {
              "type": "string",
              "format": "date-time",
              "maxLength": 40
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "grant_id",
        "state",
        "previous_state",
        "reason",
        "reservation_id",
        "operation_digest",
        "nonce_hash",
        "key_id",
        "core_boot_epoch",
        "policy_epoch",
        "sequence",
        "consumed_by_broker",
        "consumed_at"
      ],
      "additionalProperties": false,
      "allOf": [
        {
          "if": {
            "properties": {
              "state": {
                "const": "created"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  null
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "pending"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "reserved"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "pending"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "consumed"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "revoked"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created",
                  "pending",
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "expired"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created",
                  "pending",
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "invalidated"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created",
                  "pending",
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "failed"
              }
            }
          },
          "then": {
            "properties": {
              "previous_state": {
                "enum": [
                  "created",
                  "pending",
                  "reserved"
                ]
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "enum": [
                  "reserved",
                  "consumed"
                ]
              }
            }
          },
          "then": {
            "properties": {
              "reservation_id": {
                "type": "string",
                "format": "uuid"
              },
              "operation_digest": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "nonce_hash": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$"
              },
              "key_id": {
                "type": "string",
                "minLength": 1,
                "maxLength": 64
              }
            }
          }
        },
        {
          "if": {
            "properties": {
              "state": {
                "const": "consumed"
              }
            }
          },
          "then": {
            "properties": {
              "consumed_by_broker": {
                "type": "string",
                "format": "uuid"
              },
              "consumed_at": {
                "type": "string",
                "format": "date-time",
                "maxLength": 40
              }
            }
          },
          "else": {
            "properties": {
              "consumed_by_broker": {
                "type": "null"
              },
              "consumed_at": {
                "type": "null"
              }
            }
          }
        }
      ]
    },
    "permit": {
      "type": "object",
      "properties": {
        "schema_version": {
          "const": "2.0.0"
        },
        "grant_id": {
          "type": "string",
          "format": "uuid"
        },
        "reservation_id": {
          "type": "string",
          "format": "uuid"
        },
        "nonce": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "key_id": {
          "type": "string",
          "minLength": 1,
          "maxLength": 64
        },
        "core_boot_epoch": {
          "type": "string",
          "format": "uuid"
        },
        "broker_instance": {
          "type": "string",
          "format": "uuid"
        },
        "prepared_id": {
          "type": "string",
          "format": "uuid"
        },
        "invocation_id": {
          "type": "string",
          "format": "uuid"
        },
        "agent_id": {
          "enum": [
            "A01",
            "A02",
            "A03",
            "A04",
            "A05",
            "A06",
            "A07"
          ]
        },
        "session_id": {
          "type": "string",
          "format": "uuid"
        },
        "runtime_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "adapter_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "requestor_principal_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "executor_principal_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "semantic_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "operation_digest": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        },
        "policy_epoch": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "grant_epoch": {
          "type": "integer",
          "minimum": 0,
          "maximum": 9007199254740991
        },
        "expires_at": {
          "type": "string",
          "format": "date-time",
          "maxLength": 40
        },
        "ttl_ms": {
          "type": "integer",
          "minimum": 1,
          "maximum": 60000
        },
        "max_uses": {
          "const": 1
        },
        "mac": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$"
        }
      },
      "required": [
        "schema_version",
        "grant_id",
        "reservation_id",
        "nonce",
        "key_id",
        "core_boot_epoch",
        "broker_instance",
        "prepared_id",
        "invocation_id",
        "agent_id",
        "session_id",
        "runtime_digest",
        "adapter_digest",
        "requestor_principal_digest",
        "executor_principal_digest",
        "semantic_digest",
        "operation_digest",
        "policy_epoch",
        "grant_epoch",
        "expires_at",
        "ttl_ms",
        "max_uses",
        "mac"
      ],
      "additionalProperties": false
    }
  }
}
~~~

### operation-v2

<!-- ag:schema operation-v2 -->

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:operation-v2:2.0.0",
  "$ref": "urn:agentguard:types:2.0.0#/$defs/operation"
}
~~~

### permit-v2

<!-- ag:schema permit-v2 -->

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:permit-v2:2.0.0",
  "$ref": "urn:agentguard:types:2.0.0#/$defs/permit"
}
~~~

### network-grant-v2

<!-- ag:schema network-grant-v2 -->

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:network-grant-v2:2.0.0",
  "$ref": "urn:agentguard:types:2.0.0#/$defs/network_grant"
}
~~~

### capabilities-v2

<!-- ag:schema capabilities-v2 -->

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:capabilities-v2:2.0.0",
  "$ref": "urn:agentguard:types:2.0.0#/$defs/capabilities"
}
~~~

### grant-v2

<!-- ag:schema grant-v2 -->

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:grant-v2:2.0.0",
  "$ref": "urn:agentguard:types:2.0.0#/$defs/grant"
}
~~~

## 八个原始场景的2.0示例

保留只读提议、删除拒绝、MCP上传待批、豆包未知、Reasonix子Agent审批、WorkBuddy历史结果、ZCode超时和DSH结果八种场景。以下均为synthetic_fixture。历史成功声明保留claimed，不提升真实Agent证据。2.0 session UUID与原字符串的映射由Core维护，不能随意制造血缘。

### EV01

保留P0场景，全部是合成数据，迁移不继承执行权或E3/E4认证。

<!-- ag:fixture EV01 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000010",
  "agent_id": "A01",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "tool.proposed",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "Bash",
  "command": "Get-ChildItem -LiteralPath 'D:\\GuardLab\\project\\src'",
  "arguments": {
    "command": "Get-ChildItem -LiteralPath 'D:\\GuardLab\\project\\src'"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [
    {
      "path": "D:\\GuardLab\\project\\src",
      "verification": "claimed",
      "object_digest": null
    }
  ],
  "network_requests": [],
  "principal_snapshot": {
    "status": "unknown",
    "reported": null,
    "os_verified": null,
    "verifier_instance": null,
    "verified_at": null
  },
  "execution_mode": "none",
  "execution_state": "proposed",
  "policy_decision": null,
  "risk_level": "unknown",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": false,
    "single_use_grant_verified": false,
    "os_isolation_verified": false,
    "certificate": null
  },
  "grant": null,
  "broker_instance": null,
  "prepared_id": null,
  "semantic_digest": null,
  "operation_digest": null,
  "decision_id": null,
  "source": {
    "kind": "adapter",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [
    {
      "evidence_id": "00000000-0000-4000-8000-000000000100",
      "grade": "E0",
      "kind": "claimed",
      "description": "工具请求已到达，尚无执行证据",
      "artifact_digest": null
    }
  ],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### EV02

保留P0场景，全部是合成数据，迁移不继承执行权或E3/E4认证。

<!-- ag:fixture EV02 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000011",
  "agent_id": "A02",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "policy.decided",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "Bash",
  "command": "Remove-Item -LiteralPath 'D:\\GuardLab\\project\\cache' -Recurse -Force",
  "arguments": null,
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [
    {
      "path": "D:\\GuardLab\\project\\cache",
      "verification": "lexical",
      "object_digest": null
    }
  ],
  "network_requests": [],
  "principal_snapshot": {
    "status": "unknown",
    "reported": null,
    "os_verified": null,
    "verifier_instance": null,
    "verified_at": null
  },
  "execution_mode": "none",
  "execution_state": "denied",
  "policy_decision": "deny",
  "risk_level": "high",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": false,
    "single_use_grant_verified": false,
    "os_isolation_verified": false,
    "certificate": null
  },
  "grant": null,
  "broker_instance": null,
  "prepared_id": null,
  "semantic_digest": null,
  "operation_digest": null,
  "decision_id": null,
  "source": {
    "kind": "core",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [
    {
      "evidence_id": "00000000-0000-4000-8000-000000000110",
      "grade": "E0",
      "kind": "claimed",
      "description": "无有效删除授权，Core 已拒绝。宿主是否遵守需独立验证",
      "artifact_digest": null
    }
  ],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### EV03

保留P0场景，全部是合成数据，迁移不继承执行权或E3/E4认证。

<!-- ag:fixture EV03 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000012",
  "agent_id": "A03",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "mcp.called",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "mcp__files__export",
  "command": null,
  "arguments": {
    "source": "reports",
    "destination": "https://upload.example.invalid/data"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [],
  "network_requests": [
    {
      "authorization_id": null,
      "scope_id": null,
      "request_method": null,
      "scheme": "https",
      "host": "upload.example.invalid",
      "port": 443,
      "destination_ip": null,
      "dns_answers": [],
      "dns_epoch": null,
      "dns_expires_at": null,
      "resolver_id": null,
      "redirect_chain": [],
      "redirect_limit": 0,
      "proxy": {
        "mode": "unknown",
        "proxy_id": null,
        "endpoint_digest": null
      },
      "transport": "unknown",
      "path_query_digest": null,
      "upload_data_digest": null,
      "request_body_digest": null,
      "max_request_bytes": null,
      "max_response_bytes": null,
      "max_requests": null,
      "connection_id": null,
      "pool_scope": null,
      "connection_policy_epoch": null,
      "observation_state": "claimed",
      "enforcement_state": "none",
      "enforcement_scope_id": null,
      "native_permission": "unknown"
    }
  ],
  "principal_snapshot": {
    "status": "unknown",
    "reported": null,
    "os_verified": null,
    "verifier_instance": null,
    "verified_at": null
  },
  "execution_mode": "none",
  "execution_state": "pending",
  "policy_decision": "ask",
  "risk_level": "high",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": false,
    "single_use_grant_verified": false,
    "os_isolation_verified": false,
    "certificate": null
  },
  "grant": null,
  "broker_instance": null,
  "prepared_id": null,
  "semantic_digest": null,
  "operation_digest": null,
  "decision_id": null,
  "source": {
    "kind": "adapter",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [
    {
      "evidence_id": "00000000-0000-4000-8000-000000000120",
      "grade": "E0",
      "kind": "claimed",
      "description": "PTC 嵌套 MCP 调用，仍等待审批",
      "artifact_digest": null
    }
  ],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### EV04

保留P0场景，全部是合成数据，迁移不继承执行权或E3/E4认证。

<!-- ag:fixture EV04 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000013",
  "agent_id": "A04",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "protection.changed",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": null,
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "unknown",
    "reported": null,
    "os_verified": null,
    "verifier_instance": null,
    "verified_at": null
  },
  "execution_mode": "none",
  "execution_state": "unknown",
  "policy_decision": "not_evaluated",
  "risk_level": "unknown",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": false,
    "single_use_grant_verified": false,
    "os_isolation_verified": false,
    "certificate": null
  },
  "grant": null,
  "broker_instance": null,
  "prepared_id": null,
  "semantic_digest": null,
  "operation_digest": null,
  "decision_id": null,
  "source": {
    "kind": "core",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [
    {
      "evidence_id": "00000000-0000-4000-8000-000000000130",
      "grade": "E0",
      "kind": "claimed",
      "description": "豆包 App 工作模式尚无可验证接入，任务和操作未知",
      "artifact_digest": null
    }
  ],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### EV05

保留P0场景，全部是合成数据，迁移不继承执行权或E3/E4认证。

<!-- ag:fixture EV05 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000014",
  "agent_id": "A05",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "permission.requested",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "bash",
  "command": "python .\\tools\\build_report.py",
  "arguments": {
    "command": "python .\\tools\\build_report.py"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "unknown",
    "reported": null,
    "os_verified": null,
    "verifier_instance": null,
    "verified_at": null
  },
  "execution_mode": "none",
  "execution_state": "pending",
  "policy_decision": null,
  "risk_level": "unknown",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": false,
    "single_use_grant_verified": false,
    "os_isolation_verified": false,
    "certificate": null
  },
  "grant": null,
  "broker_instance": null,
  "prepared_id": null,
  "semantic_digest": null,
  "operation_digest": null,
  "decision_id": null,
  "source": {
    "kind": "adapter",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [
    {
      "evidence_id": "00000000-0000-4000-8000-000000000140",
      "grade": "E0",
      "kind": "claimed",
      "description": "Agent 称用于生成报表，脚本内容和 Windows 隔离尚未验证",
      "artifact_digest": null
    }
  ],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### EV06

保留P0场景，全部是合成数据，迁移不继承执行权或E3/E4认证。

<!-- ag:fixture EV06 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000015",
  "agent_id": "A06",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "execution.result",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "native-file-write",
  "command": null,
  "arguments": {
    "path": "report.md"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [
    {
      "path": "D:\\GuardLab\\project\\report.md",
      "verification": "claimed",
      "object_digest": null
    }
  ],
  "network_requests": [],
  "principal_snapshot": {
    "status": "unknown",
    "reported": null,
    "os_verified": null,
    "verifier_instance": null,
    "verified_at": null
  },
  "execution_mode": "none",
  "execution_state": "succeeded",
  "policy_decision": "not_evaluated",
  "risk_level": "unknown",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": false,
    "single_use_grant_verified": false,
    "os_isolation_verified": false,
    "certificate": null
  },
  "grant": null,
  "broker_instance": null,
  "prepared_id": null,
  "semantic_digest": null,
  "operation_digest": null,
  "decision_id": null,
  "source": {
    "kind": "import",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [
    {
      "evidence_id": "00000000-0000-4000-8000-000000000150",
      "grade": "E0",
      "kind": "claimed",
      "description": "导入记录声称写入成功，未独立校验文件变化，非实时前置保护",
      "artifact_digest": null
    }
  ],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### EV07

保留P0场景，全部是合成数据，迁移不继承执行权或E3/E4认证。

<!-- ag:fixture EV07 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000016",
  "agent_id": "A07",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "hook.timeout",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "Write",
  "command": null,
  "arguments": {
    "file_path": "output.txt",
    "content": "demo"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "unknown",
    "reported": null,
    "os_verified": null,
    "verifier_instance": null,
    "verified_at": null
  },
  "execution_mode": "none",
  "execution_state": "unknown",
  "policy_decision": "deny",
  "risk_level": "unknown",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": false,
    "single_use_grant_verified": false,
    "os_isolation_verified": false,
    "certificate": null
  },
  "grant": null,
  "broker_instance": null,
  "prepared_id": null,
  "semantic_digest": null,
  "operation_digest": null,
  "decision_id": null,
  "source": {
    "kind": "core",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [
    {
      "evidence_id": "00000000-0000-4000-8000-000000000160",
      "grade": "E0",
      "kind": "claimed",
      "description": "Core deadline 到期拒绝许可，ZCode 是否继续工具尚无证据",
      "artifact_digest": null
    }
  ],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### EV08

保留P0场景，全部是合成数据，迁移不继承执行权或E3/E4认证。

<!-- ag:fixture EV08 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000017",
  "agent_id": "A03",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "execution.result",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "read_file",
  "command": null,
  "arguments": {
    "path": "README.md"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "unknown",
    "reported": null,
    "os_verified": null,
    "verifier_instance": null,
    "verified_at": null
  },
  "execution_mode": "none",
  "execution_state": "succeeded",
  "policy_decision": null,
  "risk_level": "low",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": false,
    "single_use_grant_verified": false,
    "os_isolation_verified": false,
    "certificate": null
  },
  "grant": null,
  "broker_instance": null,
  "prepared_id": null,
  "semantic_digest": null,
  "operation_digest": null,
  "decision_id": null,
  "source": {
    "kind": "adapter",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [
    {
      "evidence_id": "00000000-0000-4000-8000-000000000170",
      "grade": "E0",
      "kind": "claimed",
      "description": "受认证宿主报告工具成功，仅证明来源",
      "artifact_digest": null
    },
    {
      "evidence_id": "00000000-0000-4000-8000-000000000171",
      "grade": "E0",
      "kind": "claimed",
      "description": "隔离测试根中返回内容摘要与固定夹具一致，属于未来验收示例",
      "artifact_digest": null
    }
  ],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

## 1.0迁移与向前兼容

1.0.0仅允许先用冻结event-v1验证，再以imported方式迁移。保存原event_id与原始记录摘要的关联，分配Core接收序列和合法UUID会话映射。原process_id保留为reported声明，禁止填入OS核验槽。network_destinations映射为claimed网络请求，其余新增字段unknown/null。原allow/deny均为历史来源声明，迁移后policy_decision=not_evaluated，capabilities全false，grant/digest/prepared全部null。verified_in_scope等旧字符串不转换成认证。迁移后不得自动执行。

已知2.x只在精确Schema登记和字段/枚举兼容性测试通过后接收。未知major隔离存为不可授权原始诊断，不能按较旧major猜测解析。未知安全字段拒绝，增添必填或改变语义必须major升级。新增可选诊断字段也不能覆盖安全值。

原8个legacy输入、全部正反边界夹具及CI合同见验证文档。迁移保持场景事实并清除旧授权含义，局部验证不能证明某历史事件真实发生。[验证夹具](20-PROTOCOL-VALIDATION-FIXTURES.md)

## 实现与验收

I03负责parser、Schema和字段所有权，I05负责状态与事务，I06负责最终绑定，I08负责脱敏，I21负责独立证据。AT19/21/23/24/29/34/44/45覆盖输入、伪造、缺失和状态一致性。所有运行验收待P1。

[策略](08-SECURITY-POLICY-ENGINE.md) [网络合同](17-WINDOWS-NETWORK-ISOLATION.md) [密码学](18-CRYPTOGRAPHIC-CONTRACT.md)

