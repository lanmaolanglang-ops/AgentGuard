# 统一事件协议

AgentGuard V2.0 · P0-6 · schema 1.0.0 设计稿 · 2026-11-05

## 目的

用同一 envelope 表达七款 Agent 的提议、收取、决策、执行、结果、缺口和降级。字段不足时保留 unknown/null，不补造行为。

## 结论

采用不可变事实事件与独立决策事件，通过 correlation_id 和原始 invocation 血缘关联。Adapter 只上报事实，Core 生成最终 policy_decision/risk_level/verification。事件协议不赋予执行能力，授权许可另见策略引擎。

以下 JSON Schema 及例子都是拟定合同。业务场景取自实际使用类型，所有 ID、路径、结果均为构造样例，没有真实调用或执行记录。

## 字段和语义

| 字段组 | 规则 |
|---|---|
| schema_version / event_id | 语义版本，Core 分配全局事件 UUID，重试使用源 dedup key |
| agent_id / agent_version | 仅 A01–A07，版本为真实 runtime profile 或 null，不用模型名代替 |
| session_id / parent_session_id / subagent_id | 可缺失，未知 session 不造稳定关系，Core 可另存暂态 connection scope |
| process_id / parent_process_id | OS PID 可缺，process_instance 含创建时间防 PID 复用，Agent 声明另放 extensions |
| event_type / tool_name | 类型表示收到的事实，不从“写文件工具名”推断写入成功 |
| command / arguments | 默认脱敏显示值，完整决策材料只在内存及必要的短期加密 pending 中存在 |
| working_directory / target_paths | cwd 必须包含执行上下文，路径逐项标明 claimed/lexical/handle_verified |
| network_destinations | 声明/观察/强制三种层级，域名不是稳定 IP 或上传成功证据 |
| execution_state | proposed/pending/approved/denied/dispatched/running/succeeded/failed/cancelled/unknown，与源可信度共同显示 |
| policy_decision / risk_level | Core 所有，null 或 not_evaluated 不能解释成 allow |
| evidence | 事实、声明、推断及未知分别保存，authenticated_source 不代表内容正确 |
| timestamp / received_at | 源时间可 null，Core 时间单独保留，不可信时钟不决定授权有效性 |
| correlation_id / source.invocation_id | 一个逻辑操作的多阶段相关联，不能单凭相近时间归因 |
| source / ingest_seq | 源重试/序列与 Core 接收序列分开，重连有新 instance_id |
| operation_digest / decision_id / grant_id | 指向 Core 决策记录，摘要不能当可重放授权 token |
| missing_fields / extensions | 缺失原因写入 evidence，扩展有命名空间，未知安全字段不默认许可 |
| redaction | 记录脱敏字段列表，原始 token、文件正文和命令秘密不落审计库 |

target_paths=[] 表示没有提供目标，不表示不存在文件影响。arguments=null 表示未知/不适用，arguments={} 表示已知空对象。

## JSON Schema

~~~json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentguard:event:1.0.0",
  "title": "AgentGuard normalized event",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "schema_version": {
      "const": "1.0.0"
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
      "type": [
        "string",
        "null"
      ]
    },
    "session_id": {
      "type": [
        "string",
        "null"
      ]
    },
    "process_id": {
      "type": [
        "integer",
        "null"
      ],
      "minimum": 1
    },
    "parent_process_id": {
      "type": [
        "integer",
        "null"
      ],
      "minimum": 1
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
        "grant.revoked"
      ]
    },
    "tool_name": {
      "type": [
        "string",
        "null"
      ]
    },
    "command": {
      "type": [
        "string",
        "null"
      ]
    },
    "arguments": {
      "type": [
        "object",
        "array",
        "string",
        "number",
        "boolean",
        "null"
      ]
    },
    "working_directory": {
      "type": [
        "string",
        "null"
      ]
    },
    "target_paths": {
      "type": "array",
      "maxItems": 1024,
      "items": {
        "$ref": "#/$defs/path"
      }
    },
    "network_destinations": {
      "type": "array",
      "maxItems": 256,
      "items": {
        "$ref": "#/$defs/destination"
      }
    },
    "execution_state": {
      "enum": [
        "not_applicable",
        "proposed",
        "pending",
        "approved",
        "denied",
        "dispatched",
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
        "native_fallback",
        "not_evaluated"
      ]
    },
    "risk_level": {
      "enum": [
        "low",
        "medium",
        "high",
        "critical",
        "unknown"
      ]
    },
    "evidence": {
      "type": "array",
      "maxItems": 256,
      "items": {
        "$ref": "#/$defs/evidence"
      }
    },
    "timestamp": {
      "type": [
        "string",
        "null"
      ],
      "format": "date-time"
    },
    "correlation_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "source": {
      "$ref": "#/$defs/source"
    },
    "ingest_seq": {
      "type": "integer",
      "minimum": 1
    },
    "received_at": {
      "type": "string",
      "format": "date-time"
    },
    "operation_digest": {
      "type": [
        "string",
        "null"
      ]
    },
    "missing_fields": {
      "type": "array",
      "uniqueItems": true,
      "items": {
        "type": "string"
      }
    },
    "extensions": {
      "type": "object",
      "additionalProperties": true
    },
    "decision_id": {
      "type": [
        "string",
        "null"
      ]
    },
    "grant_id": {
      "type": [
        "string",
        "null"
      ]
    },
    "parent_session_id": {
      "type": [
        "string",
        "null"
      ]
    },
    "subagent_id": {
      "type": [
        "string",
        "null"
      ]
    },
    "parent_invocation_id": {
      "type": [
        "string",
        "null"
      ]
    },
    "process_instance": {
      "type": [
        "string",
        "null"
      ]
    },
    "connection_peer": {
      "type": [
        "string",
        "null"
      ]
    },
    "policy_version": {
      "type": [
        "string",
        "null"
      ]
    },
    "redaction": {
      "type": "object",
      "required": [
        "applied",
        "fields"
      ],
      "properties": {
        "applied": {
          "type": "boolean"
        },
        "fields": {
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      },
      "additionalProperties": false
    }
  },
  "required": [
    "schema_version",
    "event_id",
    "agent_id",
    "agent_version",
    "session_id",
    "process_id",
    "parent_process_id",
    "event_type",
    "tool_name",
    "command",
    "arguments",
    "working_directory",
    "target_paths",
    "network_destinations",
    "execution_state",
    "policy_decision",
    "risk_level",
    "evidence",
    "timestamp",
    "correlation_id",
    "source",
    "ingest_seq",
    "received_at",
    "operation_digest",
    "missing_fields",
    "extensions"
  ],
  "$defs": {
    "path": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "value",
        "operation",
        "resolution",
        "identity"
      ],
      "properties": {
        "value": {
          "type": "string",
          "maxLength": 32768
        },
        "operation": {
          "enum": [
            "read",
            "create",
            "write",
            "delete",
            "rename",
            "execute",
            "unknown"
          ]
        },
        "resolution": {
          "enum": [
            "claimed",
            "lexical",
            "handle_verified",
            "unknown"
          ]
        },
        "identity": {
          "type": [
            "string",
            "null"
          ]
        }
      }
    },
    "destination": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "host",
        "port",
        "protocol",
        "verification"
      ],
      "properties": {
        "host": {
          "type": "string",
          "maxLength": 1024
        },
        "port": {
          "type": [
            "integer",
            "null"
          ],
          "minimum": 1,
          "maximum": 65535
        },
        "protocol": {
          "enum": [
            "http",
            "https",
            "tcp",
            "udp",
            "unknown"
          ]
        },
        "verification": {
          "enum": [
            "claimed",
            "observed",
            "enforced",
            "unknown"
          ]
        }
      }
    },
    "evidence": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "kind",
        "producer",
        "verification",
        "summary",
        "reference"
      ],
      "properties": {
        "kind": {
          "enum": [
            "agent_claim",
            "adapter_report",
            "core_record",
            "os_observation",
            "independent_check",
            "inference",
            "unknown"
          ]
        },
        "producer": {
          "type": "string",
          "maxLength": 128
        },
        "verification": {
          "enum": [
            "unverified",
            "authenticated_source",
            "corroborated",
            "verified_in_scope"
          ]
        },
        "summary": {
          "type": "string",
          "maxLength": 4096
        },
        "reference": {
          "type": [
            "string",
            "null"
          ]
        }
      }
    },
    "source": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "adapter_id",
        "adapter_version",
        "instance_id",
        "source_event_id",
        "sequence",
        "invocation_id",
        "channel",
        "profile_id"
      ],
      "properties": {
        "adapter_id": {
          "type": "string"
        },
        "adapter_version": {
          "type": "string"
        },
        "instance_id": {
          "type": "string"
        },
        "source_event_id": {
          "type": [
            "string",
            "null"
          ]
        },
        "sequence": {
          "type": [
            "integer",
            "null"
          ],
          "minimum": 0
        },
        "invocation_id": {
          "type": [
            "string",
            "null"
          ]
        },
        "channel": {
          "enum": [
            "hook",
            "extension",
            "import",
            "sensor",
            "core"
          ]
        },
        "profile_id": {
          "type": "string"
        }
      }
    }
  }
}
~~~

本 Schema 用于 Core 输出的 normalized event。Ingress 使用其受限投影，Core 拒绝 Adapter 提交最终 allow/risk/verified。输入实行 UTF-8、JSON 无重复键、无 NaN/Infinity、无超深对象校验，arguments 的大小和深度在传输层限制。format 必须启用断言或用显式解析补充校验。

## 事件链及状态归约

~~~mermaid
sequenceDiagram
  participant A as Agent
  participant D as Adapter
  participant C as Core
  participant U as 用户
  participant X as 执行点
  A->>D: tool.proposed
  D->>C: evaluate
  C->>C: operation.received / durable pending
  C->>U: 请求有限批准
  U->>C: 用户意图与操作摘要
  C->>C: policy.decided / grant reservation
  C->>D: permit 或 deny
  D->>X: 产品专属 verdict
  X->>C: 最终参数校验与一次性兑换
  X-->>D: started / result（能力允许时）
  D->>C: 独立阶段事件
~~~

没有最终兑换接口的 Hook profile 在收到 verdict 后只能表达“Core 返回了批准”，不能证明工具使用的是同一参数。缺少 started 证据时可直接接收 result，回放不补造真实进程启动。

同一 correlation 可以有拒绝、重新提议、最终执行等多个 invocation，不能把前一次 deny 覆盖为后一次 success。后来 OS 事实若与拒绝冲突，增加关联异常和 protection.changed，不删除拒绝记录。单一事件 execution_state 只表示该阶段快照，聚合视图由因果链决定。

## 幂等、乱序和断线

- dedup key 为 profile_instance + source_event_id，相同消息只确认一次。同键不同内容判冲突，禁止产生许可。
- 缺稳定 source ID 时全部保留，基于摘要/时间只标记可能重复，避免合并两次危险操作。
- 源 sequence 有缺口时追加 telemetry.gap。ingest_seq 保证接收排序，不能充当真实发生顺序。
- 回放按因果关系和 ingest_seq 展示，迟到的 started 可关联已到达 result，并标注迟到。
- session.ended、agent.crashed、connection.lost 不同。心跳缺失不能证明崩溃，崩溃也不能证明子进程结束。
- hook.timeout 记录 deadline 和 Core decision，host 最终行为未知时 execution_state=unknown。
- Adapter 升级产生新 instance/profile，旧 grant 失效。未知 major 只允许有界隔离诊断，不能进入授权路径。

## 脱敏与审计

完整命令的秘密值不能先落盘再脱敏。Adapter 在接触 stdout/stderr 时最小化采集，Core 二次脱敏。授权摘要对原始操作计算域分离 HMAC，日志只存摘要与脱敏表示。两个不同秘密参数即使显示为相同掩码，也必须得到不同摘要。

回放重建当时状态和解释版本，不重跑命令。链摘要针对已脱敏事件，删除保留期数据时封口日志段并记录 retention 操作，不能改写旧链。

## 真实业务类型的构造示例

场景 4 是当前无数据源的状态示例。场景 6 是未来导入格式示例，当前 WorkBuddy 导出结构仍未核实。场景 8 的独立检查也是设计夹具，未执行。

### 场景 1 · Codex 请求列出源代码目录

~~~json
{
  "schema_version": "1.0.0",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
  "agent_version": null,
  "session_id": "demo-session-A01",
  "process_id": null,
  "parent_process_id": null,
  "event_type": "tool.proposed",
  "tool_name": "Bash",
  "command": "Get-ChildItem -LiteralPath 'D:\\GuardLab\\project\\src'",
  "arguments": {
    "command": "Get-ChildItem -LiteralPath 'D:\\GuardLab\\project\\src'"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [
    {
      "value": "D:\\GuardLab\\project\\src",
      "operation": "read",
      "resolution": "claimed",
      "identity": null
    }
  ],
  "network_destinations": [],
  "execution_state": "proposed",
  "policy_decision": null,
  "risk_level": "unknown",
  "evidence": [
    {
      "kind": "adapter_report",
      "producer": "example-hook",
      "verification": "unverified",
      "summary": "工具请求已到达，尚无执行证据",
      "reference": null
    }
  ],
  "timestamp": "2026-11-05T10:00:00+08:00",
  "correlation_id": "demo-operation-1",
  "source": {
    "adapter_id": "design-example-A01",
    "adapter_version": "0.0.0-design",
    "instance_id": "example-boot-1",
    "source_event_id": "source-1",
    "sequence": 1,
    "invocation_id": "call-1",
    "channel": "hook",
    "profile_id": "unvalidated-design"
  },
  "ingest_seq": 1,
  "received_at": "2026-11-05T10:00:00.100+08:00",
  "operation_digest": null,
  "missing_fields": [
    "process_id",
    "parent_process_id",
    "agent_version"
  ],
  "extensions": {
    "example_only": true
  }
}
~~~

### 场景 2 · Claude Code 批量删除被 Core 拒绝

~~~json
{
  "schema_version": "1.0.0",
  "event_id": "00000000-0000-4000-8000-000000000002",
  "agent_id": "A02",
  "agent_version": null,
  "session_id": "demo-session-A02",
  "process_id": null,
  "parent_process_id": null,
  "event_type": "policy.decided",
  "tool_name": "Bash",
  "command": "Remove-Item -LiteralPath 'D:\\GuardLab\\project\\cache' -Recurse -Force",
  "arguments": null,
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [
    {
      "value": "D:\\GuardLab\\project\\cache",
      "operation": "delete",
      "resolution": "lexical",
      "identity": null
    }
  ],
  "network_destinations": [],
  "execution_state": "denied",
  "policy_decision": "deny",
  "risk_level": "high",
  "evidence": [
    {
      "kind": "core_record",
      "producer": "core",
      "verification": "verified_in_scope",
      "summary": "无有效删除授权，Core 已拒绝。宿主是否遵守需独立验证",
      "reference": "decision:demo-deny-2"
    }
  ],
  "timestamp": "2026-11-05T10:00:00+08:00",
  "correlation_id": "demo-operation-2",
  "source": {
    "adapter_id": "design-example-A02",
    "adapter_version": "0.0.0-design",
    "instance_id": "example-boot-1",
    "source_event_id": "source-2",
    "sequence": 2,
    "invocation_id": "call-2",
    "channel": "hook",
    "profile_id": "unvalidated-design"
  },
  "ingest_seq": 2,
  "received_at": "2026-11-05T10:00:00.100+08:00",
  "operation_digest": null,
  "missing_fields": [
    "process_id",
    "parent_process_id",
    "agent_version"
  ],
  "extensions": {
    "example_only": true
  },
  "decision_id": "demo-deny-2"
}
~~~

### 场景 3 · DSH Code Mode 嵌套 MCP 上传等待审批

~~~json
{
  "schema_version": "1.0.0",
  "event_id": "00000000-0000-4000-8000-000000000003",
  "agent_id": "A03",
  "agent_version": null,
  "session_id": "demo-session-A03",
  "process_id": null,
  "parent_process_id": null,
  "event_type": "mcp.called",
  "tool_name": "mcp__files__export",
  "command": null,
  "arguments": {
    "source": "reports",
    "destination": "https://upload.example.invalid/data"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [],
  "network_destinations": [
    {
      "host": "upload.example.invalid",
      "port": 443,
      "protocol": "https",
      "verification": "claimed"
    }
  ],
  "execution_state": "pending",
  "policy_decision": "ask",
  "risk_level": "high",
  "evidence": [
    {
      "kind": "adapter_report",
      "producer": "dsh-bridge",
      "verification": "unverified",
      "summary": "PTC 嵌套 MCP 调用，仍等待审批",
      "reference": null
    }
  ],
  "timestamp": "2026-11-05T10:00:00+08:00",
  "correlation_id": "demo-operation-3",
  "source": {
    "adapter_id": "design-example-A03",
    "adapter_version": "0.0.0-design",
    "instance_id": "example-boot-1",
    "source_event_id": "source-3",
    "sequence": 3,
    "invocation_id": "call-3",
    "channel": "hook",
    "profile_id": "unvalidated-design"
  },
  "ingest_seq": 3,
  "received_at": "2026-11-05T10:00:00.100+08:00",
  "operation_digest": null,
  "missing_fields": [
    "process_id",
    "parent_process_id",
    "agent_version"
  ],
  "extensions": {
    "example_only": true,
    "mode": "ptc",
    "transport": "run_code",
    "nested": true
  },
  "parent_invocation_id": "run-code-1"
}
~~~

### 场景 4 · 豆包工作模式尚无数据源，明确未知

~~~json
{
  "schema_version": "1.0.0",
  "event_id": "00000000-0000-4000-8000-000000000004",
  "agent_id": "A04",
  "agent_version": null,
  "session_id": null,
  "process_id": null,
  "parent_process_id": null,
  "event_type": "protection.changed",
  "tool_name": null,
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_destinations": [],
  "execution_state": "not_applicable",
  "policy_decision": "not_evaluated",
  "risk_level": "unknown",
  "evidence": [
    {
      "kind": "unknown",
      "producer": "core",
      "verification": "unverified",
      "summary": "豆包 App 工作模式尚无可验证接入，任务和操作未知",
      "reference": null
    }
  ],
  "timestamp": null,
  "correlation_id": "demo-operation-4",
  "source": {
    "adapter_id": "design-example-A04",
    "adapter_version": "0.0.0-design",
    "instance_id": "example-boot-1",
    "source_event_id": null,
    "sequence": null,
    "invocation_id": null,
    "channel": "core",
    "profile_id": "unvalidated-design"
  },
  "ingest_seq": 4,
  "received_at": "2026-11-05T10:00:00.100+08:00",
  "operation_digest": null,
  "missing_fields": [
    "agent_version",
    "session_id",
    "process_id",
    "parent_process_id",
    "timestamp",
    "tool_name",
    "command",
    "arguments",
    "working_directory"
  ],
  "extensions": {
    "example_only": true
  }
}
~~~

### 场景 5 · Reasonix 子 Agent 要运行变化中的脚本

~~~json
{
  "schema_version": "1.0.0",
  "event_id": "00000000-0000-4000-8000-000000000005",
  "agent_id": "A05",
  "agent_version": null,
  "session_id": "demo-session-A05",
  "process_id": null,
  "parent_process_id": null,
  "event_type": "permission.requested",
  "tool_name": "bash",
  "command": "python .\\tools\\build_report.py",
  "arguments": {
    "command": "python .\\tools\\build_report.py"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [],
  "network_destinations": [],
  "execution_state": "pending",
  "policy_decision": null,
  "risk_level": "unknown",
  "evidence": [
    {
      "kind": "agent_claim",
      "producer": "Reasonix child-2",
      "verification": "unverified",
      "summary": "Agent 称用于生成报表，脚本内容和 Windows 隔离尚未验证",
      "reference": null
    }
  ],
  "timestamp": "2026-11-05T10:00:00+08:00",
  "correlation_id": "demo-operation-5",
  "source": {
    "adapter_id": "design-example-A05",
    "adapter_version": "0.0.0-design",
    "instance_id": "example-boot-1",
    "source_event_id": "source-5",
    "sequence": 5,
    "invocation_id": "call-5",
    "channel": "extension",
    "profile_id": "unvalidated-design"
  },
  "ingest_seq": 5,
  "received_at": "2026-11-05T10:00:00.100+08:00",
  "operation_digest": null,
  "missing_fields": [
    "process_id",
    "parent_process_id",
    "agent_version"
  ],
  "extensions": {
    "example_only": true
  },
  "subagent_id": "child-2",
  "parent_session_id": "demo-parent-A05"
}
~~~

### 场景 6 · WorkBuddy 历史审计声称已写入报表

~~~json
{
  "schema_version": "1.0.0",
  "event_id": "00000000-0000-4000-8000-000000000006",
  "agent_id": "A06",
  "agent_version": null,
  "session_id": "demo-session-A06",
  "process_id": null,
  "parent_process_id": null,
  "event_type": "execution.result",
  "tool_name": "native-file-write",
  "command": null,
  "arguments": {
    "path": "report.md"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [
    {
      "value": "D:\\GuardLab\\project\\report.md",
      "operation": "write",
      "resolution": "claimed",
      "identity": null
    }
  ],
  "network_destinations": [],
  "execution_state": "succeeded",
  "policy_decision": "not_evaluated",
  "risk_level": "unknown",
  "evidence": [
    {
      "kind": "adapter_report",
      "producer": "user-selected-audit-export",
      "verification": "unverified",
      "summary": "导入记录声称写入成功，未独立校验文件变化，非实时前置保护",
      "reference": "import:demo-export"
    }
  ],
  "timestamp": "2026-11-05T10:00:00+08:00",
  "correlation_id": "demo-operation-6",
  "source": {
    "adapter_id": "design-example-A06",
    "adapter_version": "0.0.0-design",
    "instance_id": "example-boot-1",
    "source_event_id": "source-6",
    "sequence": 6,
    "invocation_id": "call-6",
    "channel": "import",
    "profile_id": "unvalidated-design"
  },
  "ingest_seq": 6,
  "received_at": "2026-11-05T10:00:00.100+08:00",
  "operation_digest": null,
  "missing_fields": [
    "process_id",
    "parent_process_id",
    "agent_version"
  ],
  "extensions": {
    "example_only": true
  }
}
~~~

### 场景 7 · ZCode 决策超时并出现保护缺口

~~~json
{
  "schema_version": "1.0.0",
  "event_id": "00000000-0000-4000-8000-000000000007",
  "agent_id": "A07",
  "agent_version": null,
  "session_id": "demo-session-A07",
  "process_id": null,
  "parent_process_id": null,
  "event_type": "hook.timeout",
  "tool_name": "Write",
  "command": null,
  "arguments": {
    "file_path": "output.txt",
    "content": "demo"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [],
  "network_destinations": [],
  "execution_state": "unknown",
  "policy_decision": "deny",
  "risk_level": "unknown",
  "evidence": [
    {
      "kind": "core_record",
      "producer": "core",
      "verification": "verified_in_scope",
      "summary": "Core deadline 到期拒绝许可，ZCode 是否继续工具尚无证据",
      "reference": null
    }
  ],
  "timestamp": "2026-11-05T10:00:00+08:00",
  "correlation_id": "demo-operation-7",
  "source": {
    "adapter_id": "design-example-A07",
    "adapter_version": "0.0.0-design",
    "instance_id": "example-boot-1",
    "source_event_id": "source-7",
    "sequence": 7,
    "invocation_id": "call-7",
    "channel": "hook",
    "profile_id": "unvalidated-design"
  },
  "ingest_seq": 7,
  "received_at": "2026-11-05T10:00:00.100+08:00",
  "operation_digest": null,
  "missing_fields": [
    "process_id",
    "parent_process_id",
    "agent_version"
  ],
  "extensions": {
    "example_only": true,
    "timeout_ms": 2000,
    "host_timeout_verdict": "unknown"
  }
}
~~~

### 场景 8 · 后置结果与独立夹具核验分别表达

~~~json
{
  "schema_version": "1.0.0",
  "event_id": "00000000-0000-4000-8000-000000000008",
  "agent_id": "A03",
  "agent_version": null,
  "session_id": "demo-session-A03",
  "process_id": null,
  "parent_process_id": null,
  "event_type": "execution.result",
  "tool_name": "read_file",
  "command": null,
  "arguments": {
    "path": "README.md"
  },
  "working_directory": "D:\\GuardLab\\project",
  "target_paths": [],
  "network_destinations": [],
  "execution_state": "succeeded",
  "policy_decision": null,
  "risk_level": "low",
  "evidence": [
    {
      "kind": "adapter_report",
      "producer": "dsh-result",
      "verification": "authenticated_source",
      "summary": "受认证宿主报告工具成功，仅证明来源",
      "reference": "result:8"
    },
    {
      "kind": "independent_check",
      "producer": "test-harness",
      "verification": "verified_in_scope",
      "summary": "隔离测试根中返回内容摘要与固定夹具一致，属于未来验收示例",
      "reference": "fixture:readme-v1"
    }
  ],
  "timestamp": "2026-11-05T10:00:00+08:00",
  "correlation_id": "demo-operation-8",
  "source": {
    "adapter_id": "design-example-A03",
    "adapter_version": "0.0.0-design",
    "instance_id": "example-boot-1",
    "source_event_id": "source-8",
    "sequence": 8,
    "invocation_id": "call-8",
    "channel": "hook",
    "profile_id": "unvalidated-design"
  },
  "ingest_seq": 8,
  "received_at": "2026-11-05T10:00:00.100+08:00",
  "operation_digest": null,
  "missing_fields": [
    "process_id",
    "parent_process_id",
    "agent_version"
  ],
  "extensions": {
    "example_only": true,
    "result_digest": "fixture-placeholder",
    "not_real_test": true
  }
}
~~~

## 技术依据

各 Agent 对工具名、参数、session、子 Agent 和结果的表示不同。DSH 有持久 tool/call/result，Codex Unified exec 可延后报告结果，Reasonix observation 可丢事件，WorkBuddy 候选方案可能先导入历史。[S01、S13、S26、S31–35](13-SOURCE-REGISTER.md)

## 风险

Schema 无法把不可信 Host 报告转成真实 OS 证据，时间戳和 PID 也可能伪造。脱敏后的副作用范围可能不完整，UI 必须显示不确定性。

## 尚未确定的内容

七款 source ID 的稳定性、完整映射、字段上限、进程实例数据来源、协议升级范围和落盘性能。P1 使用合成 fixture 验证协议。

