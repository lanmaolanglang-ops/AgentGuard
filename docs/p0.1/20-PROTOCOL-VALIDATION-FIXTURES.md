# 协议验证、夹具与 CI 合同

AgentGuard V2.0 P0.1 · 修订日期 2026-11-05 · AG-05 / AG-06 / AG-10

## 验证范围

本次使用现有python-jsonschema 4.26.0，Draft202012Validator、FormatChecker与本地referencing Registry。未安装软件。所有Schema先check_schema，再对全部标注Fixture按valid/invalid核验。反例以“标准验证器拒绝”为预期通过，不能描述成反例JSON满足正向Schema。

固定07的原八例为EV01–EV08，其余正例、负例、边界、legacy输入在本文件。18中CV-BUNDLE验证向量结构并另由两个语言实现比对。测试数据均为虚构，不授予七款E3/E4。

## 失败报告

runner报告fixture ID、文件、Schema、instance_path、schema_path与error message，任何预期不符、重复ID、遗漏注解和Schema错误退出非零。负例在CI也必须被拒绝。格式验证器不可关闭。引用仅本地URN，无联网取Schema。

## 验证顺序

1. 检查UTF-8、提取所有Markdown JSON注解并核对无遗漏。
2. Draft2020-12 meta-schema检查及共享defs一致性检查。
3. 所有正向、负向、边界fixture逐一检查。
4. 严格parser重复键、浮点、溢出、Unicode、深度/帧/集合限额检查。
5. 冻结1.0输入验证，再执行8个迁移，断言无继承授权、OS身份和认证。
6. 全64组grant状态对检查，另外检查scope、DNS与版本兼容性。
7. 独立Python/Node编码、frame、MAC比对全部21向量。
8. Mermaid、文件引用、任务DAG/图表与证据等级检查，生成Markdown报告。

Schema不能证明Nonce并发原子性或句柄实际类型，AT22/44/45在P1真实实现后执行。语义检查的固定向量也不能替代动态DNS/TOCTOU实验。

## CI 集成要求

I03将以下runner作为协议门禁，在每次协议/示例/Adapter映射变更的PR及主分支运行。Python依赖固定jsonschema 4.26.0及解析依赖lock，Node固定兼容24.x，Rust生产实现消费同一向量。CI无网络取Schema权限，不启动Agent，输出机器结果及Markdown错误表。任何失败阻止合并，baseline版本变更必须审查兼容性。

当前目录无Git仓库，本次只允许Markdown，因此CI工作流尚未部署。本文件提供可执行runner与明确接线任务，验收报告将“本地已验证”和“CI待I03部署”分开记录。不得声称CI已运行。

<!-- ag:validation-python -->

~~~python
import json, re, pathlib, copy, uuid, ipaddress
from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry, Resource

root = pathlib.Path(r"D:\AgentGuard\docs\p0.1")
schemas, fixtures, all_blocks = {}, [], []
for path in sorted(root.glob("*.md")):
    content = path.read_text(encoding="utf-8")
    all_blocks.extend((path.name, m.start()) for m in re.finditer(r"~~~json\s*\n", content))
    pattern = r"<!-- ag:(schema|fixture) ([^\n]*?) -->\s*\n~~~json\s*\n([\s\S]*?)\n~~~"
    for match in re.finditer(pattern, content):
        kind, annotation, raw = match.groups()
        value = json.loads(raw)
        if kind == "schema":
            if annotation in schemas:
                raise AssertionError("duplicate schema " + annotation)
            schemas[annotation] = value
        else:
            identity, schema, expected = annotation.split()
            fixtures.append((identity, schema, expected == "valid", value, path.name))
assert len(all_blocks) == len(schemas) + len(fixtures), "unannotated JSON block"
assert len({item[0] for item in fixtures}) == len(fixtures), "duplicate fixture ID"
registry = Registry()
for name, schema in schemas.items():
    Draft202012Validator.check_schema(schema)
    registry = registry.with_resource(schema["$id"], Resource.from_contents(schema))
validators = {
    name: Draft202012Validator(schema, registry=registry, format_checker=FormatChecker())
    for name, schema in schemas.items()
}
assert schemas["types-v2"]["$defs"] == schemas["event-v2"]["$defs"]
failures = []
for identity, schema, expected, value, path in fixtures:
    errors = sorted(validators[schema].iter_errors(value), key=lambda e: str(list(e.path)))
    if bool(errors) == expected:
        failures.append({
            "fixture": identity, "schema": schema, "file": path,
            "expected": "valid" if expected else "invalid",
            "errors": [{"instance_path": list(e.path), "schema_path": list(e.schema_path), "message": e.message} for e in errors[:4]]
        })
assert not failures, json.dumps(failures, ensure_ascii=False)
data = {item[0]: item[3] for item in fixtures}

def strict_parse(raw):
    if len(raw.encode("utf-8", "strict")) > 1048576:
        raise ValueError("frame exceeds 1MiB")
    if raw.startswith("\ufeff"):
        raise ValueError("BOM")
    def pairs(items):
        output = {}
        for key, value in items:
            if key in output:
                raise ValueError("duplicate key")
            output[key] = value
        return output
    def integral(text):
        value = int(text)
        if text == "-0" or abs(value) > 9007199254740991:
            raise ValueError("integer range")
        return value
    def reject(text):
        raise ValueError("noninteger")
    value = json.loads(raw, object_pairs_hook=pairs, parse_int=integral, parse_float=reject, parse_constant=reject)
    def walk(item, depth=0):
        if depth > 8:
            raise ValueError("depth")
        if isinstance(item, str):
            item.encode("utf-8", "strict")
        elif isinstance(item, list):
            if len(item) > 128:
                raise ValueError("array budget")
            for child in item:
                walk(child, depth + 1)
        elif isinstance(item, dict):
            if len(item) > 64:
                raise ValueError("map budget")
            for key, child in item.items():
                key.encode("utf-8", "strict")
                walk(child, depth + 1)
    walk(value)
    return value

bad_raw = [
    '{"a":1,"a":2}', '{"a":1.0}', '{"a":1e0}', '{"a":-0}',
    '{"a":NaN}', '{"a":Infinity}', '{"a":9007199254740992}',
    '"\\ud800"', '\ufeff{}', "[" * 10 + "0" + "]" * 10,
    json.dumps({"x": "a" * 1048576}), json.dumps(list(range(129))),
    json.dumps({str(i): i for i in range(65)})
]
for raw in bad_raw:
    try:
        strict_parse(raw)
    except (ValueError, UnicodeError):
        pass
    else:
        raise AssertionError("strict parser accepted invalid input")
assert strict_parse('{"a":null,"b":"","c":9007199254740991}')["c"] == 9007199254740991

def migrate(old):
    validators["event-v1"].validate(old)
    event = copy.deepcopy(data["EV01"])
    event.update({
        "schema_version": "2.0.0", "record_kind": "imported",
        "event_id": old["event_id"], "agent_id": old["agent_id"],
        "agent_version": old["agent_version"], "event_type": old["event_type"],
        "timestamp": old["timestamp"], "received_at": "2026-11-05T08:00:00Z",
        "command": old["command"], "arguments": old["arguments"],
        "tool_name": old["tool_name"], "working_directory": old["working_directory"],
        "policy_decision": "not_evaluated", "risk_level": "unknown",
        "operation_digest": None, "semantic_digest": None, "grant": None,
        "broker_instance": None, "prepared_id": None, "decision_id": None,
        "execution_mode": "none", "parent_invocation_id": None,
        "subagent_id": old.get("subagent_id"), "depth": 0, "provider_id": None,
        "target_paths": [{"path": item["value"], "verification": "claimed", "object_digest": None} for item in old["target_paths"]],
        "network_requests": [], "capabilities": copy.deepcopy(data["CP00"]),
        "execution_state": old["execution_state"] if old["execution_state"] in ["proposed", "pending", "denied", "running", "succeeded", "failed", "cancelled", "unknown"] else "unknown",
        "missing_fields": ["principal_snapshot.os_verified", "migration.runtime_certification"],
        "extensions": {}
    })
    # Namespace mapping preserves imported session grouping, not OS authentication.
    for field in ["session_id", "parent_session_id"]:
        source = old.get(field)
        event[field] = str(uuid.uuid5(uuid.NAMESPACE_URL, "agentguard-import:" + source)) if source else None
    event["correlation_id"] = str(uuid.uuid5(uuid.NAMESPACE_URL, "agentguard-import:" + old["correlation_id"]))
    event["source"] = {
        "kind": "import", "instance_id": "00000000-0000-4000-8000-000000000999",
        "sequence": old["ingest_seq"], "invocation_id": None,
        "authenticated": False, "source_event_id": "legacy-" + old["event_id"]
    }
    event["principal_snapshot"] = {
        "status": "claimed" if old["process_id"] else "unknown",
        "reported": {"product": old["agent_id"], "version": old["agent_version"], "pid": old["process_id"], "sid": None} if old["process_id"] else None,
        "os_verified": None, "verifier_instance": None, "verified_at": None
    }
    event["evidence"] = [
        {"evidence_id": str(uuid.uuid5(uuid.NAMESPACE_URL, old["event_id"] + str(index))),
         "grade": "E0", "kind": "claimed", "description": item["summary"], "artifact_digest": None}
        for index, item in enumerate(old["evidence"])
    ]
    for item in old["network_destinations"]:
        network = copy.deepcopy(data["NW01"])
        for key in ["authorization_id", "scope_id", "destination_ip", "dns_epoch", "dns_expires_at", "resolver_id", "path_query_digest", "upload_data_digest", "request_body_digest", "max_request_bytes", "max_response_bytes", "max_requests", "connection_id", "pool_scope", "connection_policy_epoch", "enforcement_scope_id"]:
            network[key] = None
        network.update({
            "request_method": None, "host": item["host"], "port": item["port"],
            "scheme": item["protocol"] if item["protocol"] in ["http", "https"] else "other",
            "dns_answers": [], "redirect_chain": [], "redirect_limit": 0,
            "proxy": {"mode": "unknown", "proxy_id": None, "endpoint_digest": None},
            "transport": "unknown", "observation_state": "claimed",
            "enforcement_state": "none", "native_permission": "unknown"
        })
        event["network_requests"].append(network)
    validators["event-v2"].validate(event)
    assert event["grant"] is None and event["operation_digest"] is None
    assert not any(event["capabilities"][flag] for flag in schemas["types-v2"]["$defs"]["flagset"]["properties"])
    return event

migrations = [migrate(data["LEG" + str(i).zfill(2)]) for i in range(1, 9)]
assert migrations[2]["network_requests"][0]["host"] == "upload.example.invalid"
assert migrations[5]["evidence"][0]["kind"] == "claimed"

states = ["created", "pending", "reserved", "consumed", "revoked", "expired", "invalidated", "failed"]
allowed = {("created", "pending"), ("pending", "reserved"), ("reserved", "consumed")}
allowed.update((before, after) for before in states[:3] for after in states[4:])
for before in states:
    for after in states:
        grant = copy.deepcopy(data["GR-" + after])
        grant["previous_state"] = before
        valid = validators["grant-v2"].is_valid(grant)
        assert valid == ((before, after) in allowed), (before, after)

def semantic_capability(cap):
    certificate = cap["certificate"]
    return certificate is None or cap["scope_id"] == certificate["claim_scope_id"]
assert semantic_capability(data["CP01"])
invalid_scope = copy.deepcopy(data["CP01"])
invalid_scope["scope_id"] = "00000000-0000-4000-8000-000000009999"
assert validators["capabilities-v2"].is_valid(invalid_scope)
assert not semantic_capability(invalid_scope)
def semantic_network(net):
    return net["destination_ip"] in [item["ip"] for item in net["dns_answers"]] and all(ipaddress.ip_address(item["ip"]).version == item["family"] for item in net["dns_answers"])
assert semantic_network(data["NW01"])
invalid_dns = copy.deepcopy(data["NW01"])
invalid_dns["destination_ip"] = "192.0.2.2"
assert validators["network-grant-v2"].is_valid(invalid_dns)
assert not semantic_network(invalid_dns)
bad_family = copy.deepcopy(data["NW01"])
bad_family["dns_answers"][0]["family"] = 6
assert not semantic_network(bad_family)
for version in ["1.0.0", "2.1.0", "3.0.0"]:
    event = copy.deepcopy(data["EV01"])
    event["schema_version"] = version
    assert not validators["event-v2"].is_valid(event)
print(json.dumps({
    "schemas": len(schemas), "fixtures": len(fixtures),
    "positive": sum(item[2] for item in fixtures),
    "negative": sum(not item[2] for item in fixtures),
    "fixture_failures": 0, "legacy_migrations": len(migrations),
    "state_pair_checks": 64, "parser_rejections": len(bad_raw),
    "parser_positive": 1, "semantic_checks": 6, "version_rejections": 3,
    "ci_deployed": False
}))

~~~

## 本地运行入口

PowerShell读取本文件中ag:validation-python后的python围栏，交给已有解释器。CI在自己的checkout将root参数替换为仓库内docs/p0.1。示例代码自身不创建文件。

~~~powershell
$document = [IO.File]::ReadAllText('D:\AgentGuard\docs\p0.1\20-PROTOCOL-VALIDATION-FIXTURES.md')
$match = [regex]::Match($document, '(?s)<!-- ag:validation-python -->\s*\n~~~python\s*\n(.*?)\n~~~')
if (-not $match.Success) { throw 'validation runner missing' }
$code = $match.Groups[1].Value
& 'C:\Users\烧冬瓜的Mac\.codex\skills\ppt-master\.venv\Scripts\python.exe' -B -c $code
if ($LASTEXITCODE -ne 0) { throw 'protocol validation failed' }
~~~

## 冻结的1.0输入Schema

仅用于导入验证。不能作为新版本授权接口。

<!-- ag:schema event-v1 -->

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

### LEG01

冻结的1.0.0迁移输入。预期通过对应Schema。

<!-- ag:fixture LEG01 event-v1 valid -->

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

### LEG02

冻结的1.0.0迁移输入。预期通过对应Schema。

<!-- ag:fixture LEG02 event-v1 valid -->

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

### LEG03

冻结的1.0.0迁移输入。预期通过对应Schema。

<!-- ag:fixture LEG03 event-v1 valid -->

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

### LEG04

冻结的1.0.0迁移输入。预期通过对应Schema。

<!-- ag:fixture LEG04 event-v1 valid -->

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

### LEG05

冻结的1.0.0迁移输入。预期通过对应Schema。

<!-- ag:fixture LEG05 event-v1 valid -->

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

### LEG06

冻结的1.0.0迁移输入。预期通过对应Schema。

<!-- ag:fixture LEG06 event-v1 valid -->

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

### LEG07

冻结的1.0.0迁移输入。预期通过对应Schema。

<!-- ag:fixture LEG07 event-v1 valid -->

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

### LEG08

冻结的1.0.0迁移输入。预期通过对应Schema。

<!-- ag:fixture LEG08 event-v1 valid -->

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

### EV09

强绑定与单次许可的合成正例，无真实认证。预期通过对应Schema。

<!-- ag:fixture EV09 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "os_verified",
    "reported": null,
    "os_verified": {
      "windows_sid": "S-1-5-21-100-200-300-1001",
      "logon_luid": "0x0000000000001234",
      "integrity_level": "low",
      "token_restrictions": {
        "is_restricted": true,
        "restricting_sids": [
          "S-1-15-2-123"
        ],
        "deny_only_sids": [],
        "removed_privileges": [
          "SeDebugPrivilege"
        ],
        "modified_luid": "0x0000000000002345",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "process_instance": {
        "pid": 1000,
        "created_filetime": "133000000000000000",
        "os_boot_id": "00000000-0000-4000-8000-000000000500"
      },
      "parent_process_instance": null,
      "sandbox_identity": {
        "mechanism": "restricted_token",
        "identity_id": "fixture-worker",
        "capability_sids": [],
        "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "agent_runtime_identity": {
        "product": "synthetic-worker",
        "version": "fixture-1",
        "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "generation": "00000000-0000-4000-8000-000000000501"
      }
    },
    "verifier_instance": "00000000-0000-4000-8000-000000000606",
    "verified_at": "2026-11-05T08:00:00Z"
  },
  "execution_mode": "B",
  "execution_state": "prepared",
  "policy_decision": "allow",
  "risk_level": "low",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": true,
    "single_use_grant_verified": true,
    "os_isolation_verified": false,
    "certificate": {
      "certificate_id": "00000000-0000-4000-8000-000000000901",
      "claim_scope_id": "00000000-0000-4000-8000-000000000900",
      "agent_id": "A01",
      "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "entrypoint_ids": [
        "fixture-read"
      ],
      "execution_mode": "B",
      "resource_scope": [
        "fixture-tree"
      ],
      "flags": {
        "telemetry_verified": false,
        "pre_execution_block_verified": false,
        "parameter_binding_verified": true,
        "single_use_grant_verified": true,
        "os_isolation_verified": false
      },
      "evidence_refs": [
        "00000000-0000-4000-8000-000000000902"
      ],
      "evidence_grade": "E3",
      "os_build": "synthetic-26200",
      "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "issued_at": "2026-11-05T08:00:00Z",
      "expires_at": "2026-11-05T09:00:00Z",
      "verified_by": "00000000-0000-4000-8000-000000000903",
      "fault_behavior": "verified_fail_closed",
      "effective_state": "valid"
    }
  },
  "grant": {
    "grant_id": "00000000-0000-4000-8000-000000000600",
    "state": "reserved",
    "previous_state": "pending",
    "reason": "synthetic",
    "reservation_id": "00000000-0000-4000-8000-000000000601",
    "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "key_id": "fixture-key",
    "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
    "policy_epoch": 1,
    "sequence": 3,
    "consumed_by_broker": null,
    "consumed_at": null
  },
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "decision_id": "00000000-0000-4000-8000-000000000605",
  "source": {
    "kind": "core",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [],
  "missing_fields": [],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### EV10

消费与执行事件分开。预期通过对应Schema。

<!-- ag:fixture EV10 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "execution.started",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "os_verified",
    "reported": null,
    "os_verified": {
      "windows_sid": "S-1-5-21-100-200-300-1001",
      "logon_luid": "0x0000000000001234",
      "integrity_level": "low",
      "token_restrictions": {
        "is_restricted": true,
        "restricting_sids": [
          "S-1-15-2-123"
        ],
        "deny_only_sids": [],
        "removed_privileges": [
          "SeDebugPrivilege"
        ],
        "modified_luid": "0x0000000000002345",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "process_instance": {
        "pid": 1000,
        "created_filetime": "133000000000000000",
        "os_boot_id": "00000000-0000-4000-8000-000000000500"
      },
      "parent_process_instance": null,
      "sandbox_identity": {
        "mechanism": "restricted_token",
        "identity_id": "fixture-worker",
        "capability_sids": [],
        "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "agent_runtime_identity": {
        "product": "synthetic-worker",
        "version": "fixture-1",
        "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "generation": "00000000-0000-4000-8000-000000000501"
      }
    },
    "verifier_instance": "00000000-0000-4000-8000-000000000606",
    "verified_at": "2026-11-05T08:00:00Z"
  },
  "execution_mode": "B",
  "execution_state": "running",
  "policy_decision": null,
  "risk_level": "low",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": true,
    "single_use_grant_verified": true,
    "os_isolation_verified": false,
    "certificate": {
      "certificate_id": "00000000-0000-4000-8000-000000000901",
      "claim_scope_id": "00000000-0000-4000-8000-000000000900",
      "agent_id": "A01",
      "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "entrypoint_ids": [
        "fixture-read"
      ],
      "execution_mode": "B",
      "resource_scope": [
        "fixture-tree"
      ],
      "flags": {
        "telemetry_verified": false,
        "pre_execution_block_verified": false,
        "parameter_binding_verified": true,
        "single_use_grant_verified": true,
        "os_isolation_verified": false
      },
      "evidence_refs": [
        "00000000-0000-4000-8000-000000000902"
      ],
      "evidence_grade": "E3",
      "os_build": "synthetic-26200",
      "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "issued_at": "2026-11-05T08:00:00Z",
      "expires_at": "2026-11-05T09:00:00Z",
      "verified_by": "00000000-0000-4000-8000-000000000903",
      "fault_behavior": "verified_fail_closed",
      "effective_state": "valid"
    }
  },
  "grant": {
    "grant_id": "00000000-0000-4000-8000-000000000600",
    "state": "consumed",
    "previous_state": "reserved",
    "reason": "synthetic",
    "reservation_id": "00000000-0000-4000-8000-000000000601",
    "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "key_id": "fixture-key",
    "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
    "policy_epoch": 1,
    "sequence": 4,
    "consumed_by_broker": "00000000-0000-4000-8000-000000000603",
    "consumed_at": "2026-11-05T08:00:00Z"
  },
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "decision_id": "00000000-0000-4000-8000-000000000605",
  "source": {
    "kind": "broker",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [],
  "missing_fields": [],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### OP01

短期权威快照，路径为虚构，无运行。预期通过对应Schema。

<!-- ag:fixture OP01 operation-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "agent_id": "A01",
  "runtime": {
    "product": "synthetic-worker",
    "version": "fixture-1",
    "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "generation": "00000000-0000-4000-8000-000000000501"
  },
  "adapter_id": "synthetic-adapter",
  "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "provider_id": "synthetic-provider",
  "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "session_id": "00000000-0000-4000-8000-000000000701",
  "invocation_id": "00000000-0000-4000-8000-000000000702",
  "requestor_principal": {
    "windows_sid": "S-1-5-21-100-200-300-1001",
    "logon_luid": "0x0000000000001234",
    "integrity_level": "low",
    "token_restrictions": {
      "is_restricted": true,
      "restricting_sids": [
        "S-1-15-2-123"
      ],
      "deny_only_sids": [],
      "removed_privileges": [
        "SeDebugPrivilege"
      ],
      "modified_luid": "0x0000000000002345",
      "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "process_instance": {
      "pid": 1000,
      "created_filetime": "133000000000000000",
      "os_boot_id": "00000000-0000-4000-8000-000000000500"
    },
    "parent_process_instance": null,
    "sandbox_identity": {
      "mechanism": "restricted_token",
      "identity_id": "fixture-worker",
      "capability_sids": [],
      "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "agent_runtime_identity": {
      "product": "synthetic-worker",
      "version": "fixture-1",
      "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "generation": "00000000-0000-4000-8000-000000000501"
    }
  },
  "executor_principal": {
    "windows_sid": "S-1-5-21-100-200-300-1001",
    "logon_luid": "0x0000000000001234",
    "integrity_level": "low",
    "token_restrictions": {
      "is_restricted": true,
      "restricting_sids": [
        "S-1-15-2-123"
      ],
      "deny_only_sids": [],
      "removed_privileges": [
        "SeDebugPrivilege"
      ],
      "modified_luid": "0x0000000000002345",
      "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "process_instance": {
      "pid": 1000,
      "created_filetime": "133000000000000000",
      "os_boot_id": "00000000-0000-4000-8000-000000000500"
    },
    "parent_process_instance": null,
    "sandbox_identity": {
      "mechanism": "restricted_token",
      "identity_id": "fixture-worker",
      "capability_sids": [],
      "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "agent_runtime_identity": {
      "product": "synthetic-worker",
      "version": "fixture-1",
      "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "generation": "00000000-0000-4000-8000-000000000501"
    }
  },
  "recipe": "file.read.v1",
  "command": null,
  "arguments": {
    "target_index": 0,
    "max_bytes": 4096
  },
  "working_directory": {
    "path": "D:\\GuardLab",
    "volume_id": "fixture-volume",
    "file_id": "dir1",
    "content_digest": null,
    "kind": "directory",
    "verification": "handle_verified"
  },
  "targets": [
    {
      "path": "D:\\GuardLab\\example.txt",
      "volume_id": "fixture-volume",
      "file_id": "file1",
      "content_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "kind": "file",
      "verification": "handle_verified"
    }
  ],
  "executable_digest": null,
  "script_digest": null,
  "input_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "environment_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "stdin_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "native_permission": "allow",
  "network_authorization": null,
  "secret_parameters": {
    "key_id": "fixture-secret",
    "entries": []
  },
  "handle_manifest": [
    {
      "handle_ref": "00000000-0000-4000-8000-000000000703",
      "object_type": "file",
      "object_identity_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "granted_access": 1179785,
      "inherit_to_worker": false,
      "purpose": "target"
    }
  ]
}
~~~

### PM01

MAC仅为语法占位，密码学验证必须另验。预期通过对应Schema。

<!-- ag:fixture PM01 permit-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "nonce": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "invocation_id": "00000000-0000-4000-8000-000000000702",
  "agent_id": "A01",
  "session_id": "00000000-0000-4000-8000-000000000701",
  "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "requestor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "executor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "policy_epoch": 1,
  "grant_epoch": 1,
  "expires_at": "2026-11-05T08:00:30Z",
  "ttl_ms": 30000,
  "max_uses": 1,
  "mac": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
~~~

### NW01

文档地址，未访问。预期通过对应Schema。

<!-- ag:fixture NW01 network-grant-v2 valid -->

~~~json
{
  "authorization_id": "00000000-0000-4000-8000-000000000801",
  "scope_id": "00000000-0000-4000-8000-000000000802",
  "request_method": "GET",
  "scheme": "https",
  "host": "sink.invalid",
  "port": 443,
  "destination_ip": "192.0.2.1",
  "dns_answers": [
    {
      "ip": "192.0.2.1",
      "family": 4
    }
  ],
  "dns_epoch": "00000000-0000-4000-8000-000000000803",
  "dns_expires_at": "2026-11-05T08:00:30Z",
  "resolver_id": "fixture-resolver",
  "redirect_chain": [],
  "redirect_limit": 0,
  "proxy": {
    "mode": "broker",
    "proxy_id": "fixture-network",
    "endpoint_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  },
  "transport": "tcp",
  "path_query_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "upload_data_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "request_body_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "max_request_bytes": 1024,
  "max_response_bytes": 4096,
  "max_requests": 1,
  "connection_id": "00000000-0000-4000-8000-000000000804",
  "pool_scope": "00000000-0000-4000-8000-000000000805",
  "connection_policy_epoch": 1,
  "observation_state": "request_seen",
  "enforcement_state": "broker_egress",
  "enforcement_scope_id": "00000000-0000-4000-8000-000000000802",
  "native_permission": "allow"
}
~~~

### EV11

网络观察与请求授权分开。预期通过对应Schema。

<!-- ag:fixture EV11 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "network.observed",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [
    {
      "authorization_id": "00000000-0000-4000-8000-000000000801",
      "scope_id": "00000000-0000-4000-8000-000000000802",
      "request_method": "GET",
      "scheme": "https",
      "host": "sink.invalid",
      "port": 443,
      "destination_ip": "192.0.2.1",
      "dns_answers": [
        {
          "ip": "192.0.2.1",
          "family": 4
        }
      ],
      "dns_epoch": "00000000-0000-4000-8000-000000000803",
      "dns_expires_at": "2026-11-05T08:00:30Z",
      "resolver_id": "fixture-resolver",
      "redirect_chain": [],
      "redirect_limit": 0,
      "proxy": {
        "mode": "broker",
        "proxy_id": "fixture-network",
        "endpoint_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "transport": "tcp",
      "path_query_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "upload_data_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "request_body_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "max_request_bytes": 1024,
      "max_response_bytes": 4096,
      "max_requests": 1,
      "connection_id": "00000000-0000-4000-8000-000000000804",
      "pool_scope": "00000000-0000-4000-8000-000000000805",
      "connection_policy_epoch": 1,
      "observation_state": "request_seen",
      "enforcement_state": "broker_egress",
      "enforcement_scope_id": "00000000-0000-4000-8000-000000000802",
      "native_permission": "allow"
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
  "evidence": [],
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

### CP01

合成L2有限deny，参数与单次标志false。预期通过对应Schema。

<!-- ag:fixture CP01 capabilities-v2 valid -->

~~~json
{
  "scope_id": "00000000-0000-4000-8000-000000000900",
  "level": "L2",
  "telemetry_verified": false,
  "pre_execution_block_verified": true,
  "parameter_binding_verified": false,
  "single_use_grant_verified": false,
  "os_isolation_verified": false,
  "certificate": {
    "certificate_id": "00000000-0000-4000-8000-000000000901",
    "claim_scope_id": "00000000-0000-4000-8000-000000000900",
    "agent_id": "A01",
    "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "entrypoint_ids": [
      "fixture-read"
    ],
    "execution_mode": "A",
    "resource_scope": [
      "fixture-tree"
    ],
    "flags": {
      "telemetry_verified": false,
      "pre_execution_block_verified": true,
      "parameter_binding_verified": false,
      "single_use_grant_verified": false,
      "os_isolation_verified": false
    },
    "evidence_refs": [
      "00000000-0000-4000-8000-000000000902"
    ],
    "evidence_grade": "E3",
    "os_build": "synthetic-26200",
    "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "issued_at": "2026-11-05T08:00:00Z",
    "expires_at": "2026-11-05T09:00:00Z",
    "verified_by": "00000000-0000-4000-8000-000000000903",
    "fault_behavior": "known_fail_open",
    "effective_state": "valid"
  }
}
~~~

### CP02

合成C隔离，无Hook认证。预期通过对应Schema。

<!-- ag:fixture CP02 capabilities-v2 valid -->

~~~json
{
  "scope_id": "00000000-0000-4000-8000-000000000900",
  "level": "L3",
  "telemetry_verified": false,
  "pre_execution_block_verified": false,
  "parameter_binding_verified": false,
  "single_use_grant_verified": false,
  "os_isolation_verified": true,
  "certificate": {
    "certificate_id": "00000000-0000-4000-8000-000000000901",
    "claim_scope_id": "00000000-0000-4000-8000-000000000900",
    "agent_id": "A01",
    "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "entrypoint_ids": [
      "fixture-read"
    ],
    "execution_mode": "C",
    "resource_scope": [
      "fixture-tree"
    ],
    "flags": {
      "telemetry_verified": false,
      "pre_execution_block_verified": false,
      "parameter_binding_verified": false,
      "single_use_grant_verified": false,
      "os_isolation_verified": true
    },
    "evidence_refs": [
      "00000000-0000-4000-8000-000000000902"
    ],
    "evidence_grade": "E4",
    "os_build": "synthetic-26200",
    "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "issued_at": "2026-11-05T08:00:00Z",
    "expires_at": "2026-11-05T09:00:00Z",
    "verified_by": "00000000-0000-4000-8000-000000000903",
    "fault_behavior": "verified_fail_closed",
    "effective_state": "valid"
  }
}
~~~

### NEG01

缺失身份快照。预期被对应Schema拒绝。

<!-- ag:fixture NEG01 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [],
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
  "evidence": [],
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

### NEG02

扩展影射安全字段。预期被对应Schema拒绝。

<!-- ag:fixture NEG02 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
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
  "evidence": [],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {
    "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  }
}
~~~

### NEG03

诊断超长。预期被对应Schema拒绝。

<!-- ag:fixture NEG03 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
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
  "evidence": [],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {
    "diagnostic.extra": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
  }
}
~~~

### NEG04

未知执行模式。预期被对应Schema拒绝。

<!-- ag:fixture NEG04 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
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
  "execution_mode": "D",
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
  "evidence": [],
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

### NEG05

未知major。预期被对应Schema拒绝。

<!-- ag:fixture NEG05 event-v2 invalid -->

~~~json
{
  "schema_version": "3.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
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
  "evidence": [],
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

### NEG06

A不返回自动allow。预期被对应Schema拒绝。

<!-- ag:fixture NEG06 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "os_verified",
    "reported": null,
    "os_verified": {
      "windows_sid": "S-1-5-21-100-200-300-1001",
      "logon_luid": "0x0000000000001234",
      "integrity_level": "low",
      "token_restrictions": {
        "is_restricted": true,
        "restricting_sids": [
          "S-1-15-2-123"
        ],
        "deny_only_sids": [],
        "removed_privileges": [
          "SeDebugPrivilege"
        ],
        "modified_luid": "0x0000000000002345",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "process_instance": {
        "pid": 1000,
        "created_filetime": "133000000000000000",
        "os_boot_id": "00000000-0000-4000-8000-000000000500"
      },
      "parent_process_instance": null,
      "sandbox_identity": {
        "mechanism": "restricted_token",
        "identity_id": "fixture-worker",
        "capability_sids": [],
        "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "agent_runtime_identity": {
        "product": "synthetic-worker",
        "version": "fixture-1",
        "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "generation": "00000000-0000-4000-8000-000000000501"
      }
    },
    "verifier_instance": "00000000-0000-4000-8000-000000000606",
    "verified_at": "2026-11-05T08:00:00Z"
  },
  "execution_mode": "A",
  "execution_state": "prepared",
  "policy_decision": "allow",
  "risk_level": "low",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": true,
    "single_use_grant_verified": true,
    "os_isolation_verified": false,
    "certificate": {
      "certificate_id": "00000000-0000-4000-8000-000000000901",
      "claim_scope_id": "00000000-0000-4000-8000-000000000900",
      "agent_id": "A01",
      "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "entrypoint_ids": [
        "fixture-read"
      ],
      "execution_mode": "B",
      "resource_scope": [
        "fixture-tree"
      ],
      "flags": {
        "telemetry_verified": false,
        "pre_execution_block_verified": false,
        "parameter_binding_verified": true,
        "single_use_grant_verified": true,
        "os_isolation_verified": false
      },
      "evidence_refs": [
        "00000000-0000-4000-8000-000000000902"
      ],
      "evidence_grade": "E3",
      "os_build": "synthetic-26200",
      "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "issued_at": "2026-11-05T08:00:00Z",
      "expires_at": "2026-11-05T09:00:00Z",
      "verified_by": "00000000-0000-4000-8000-000000000903",
      "fault_behavior": "verified_fail_closed",
      "effective_state": "valid"
    }
  },
  "grant": {
    "grant_id": "00000000-0000-4000-8000-000000000600",
    "state": "reserved",
    "previous_state": "pending",
    "reason": "synthetic",
    "reservation_id": "00000000-0000-4000-8000-000000000601",
    "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "key_id": "fixture-key",
    "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
    "policy_epoch": 1,
    "sequence": 3,
    "consumed_by_broker": null,
    "consumed_at": null
  },
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "decision_id": "00000000-0000-4000-8000-000000000605",
  "source": {
    "kind": "core",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [],
  "missing_fields": [],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### NEG07

自报不能占OS槽。预期被对应Schema拒绝。

<!-- ag:fixture NEG07 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "claimed",
    "reported": null,
    "os_verified": {
      "windows_sid": "S-1-5-21-100-200-300-1001",
      "logon_luid": "0x0000000000001234",
      "integrity_level": "low",
      "token_restrictions": {
        "is_restricted": true,
        "restricting_sids": [
          "S-1-15-2-123"
        ],
        "deny_only_sids": [],
        "removed_privileges": [
          "SeDebugPrivilege"
        ],
        "modified_luid": "0x0000000000002345",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "process_instance": {
        "pid": 1000,
        "created_filetime": "133000000000000000",
        "os_boot_id": "00000000-0000-4000-8000-000000000500"
      },
      "parent_process_instance": null,
      "sandbox_identity": {
        "mechanism": "restricted_token",
        "identity_id": "fixture-worker",
        "capability_sids": [],
        "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "agent_runtime_identity": {
        "product": "synthetic-worker",
        "version": "fixture-1",
        "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "generation": "00000000-0000-4000-8000-000000000501"
      }
    },
    "verifier_instance": "00000000-0000-4000-8000-000000000606",
    "verified_at": "2026-11-05T08:00:00Z"
  },
  "execution_mode": "B",
  "execution_state": "prepared",
  "policy_decision": "allow",
  "risk_level": "low",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": true,
    "single_use_grant_verified": true,
    "os_isolation_verified": false,
    "certificate": {
      "certificate_id": "00000000-0000-4000-8000-000000000901",
      "claim_scope_id": "00000000-0000-4000-8000-000000000900",
      "agent_id": "A01",
      "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "entrypoint_ids": [
        "fixture-read"
      ],
      "execution_mode": "B",
      "resource_scope": [
        "fixture-tree"
      ],
      "flags": {
        "telemetry_verified": false,
        "pre_execution_block_verified": false,
        "parameter_binding_verified": true,
        "single_use_grant_verified": true,
        "os_isolation_verified": false
      },
      "evidence_refs": [
        "00000000-0000-4000-8000-000000000902"
      ],
      "evidence_grade": "E3",
      "os_build": "synthetic-26200",
      "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "issued_at": "2026-11-05T08:00:00Z",
      "expires_at": "2026-11-05T09:00:00Z",
      "verified_by": "00000000-0000-4000-8000-000000000903",
      "fault_behavior": "verified_fail_closed",
      "effective_state": "valid"
    }
  },
  "grant": {
    "grant_id": "00000000-0000-4000-8000-000000000600",
    "state": "reserved",
    "previous_state": "pending",
    "reason": "synthetic",
    "reservation_id": "00000000-0000-4000-8000-000000000601",
    "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "key_id": "fixture-key",
    "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
    "policy_epoch": 1,
    "sequence": 3,
    "consumed_by_broker": null,
    "consumed_at": null
  },
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "decision_id": "00000000-0000-4000-8000-000000000605",
  "source": {
    "kind": "core",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [],
  "missing_fields": [],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### NEG08

allow缺摘要。预期被对应Schema拒绝。

<!-- ag:fixture NEG08 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "os_verified",
    "reported": null,
    "os_verified": {
      "windows_sid": "S-1-5-21-100-200-300-1001",
      "logon_luid": "0x0000000000001234",
      "integrity_level": "low",
      "token_restrictions": {
        "is_restricted": true,
        "restricting_sids": [
          "S-1-15-2-123"
        ],
        "deny_only_sids": [],
        "removed_privileges": [
          "SeDebugPrivilege"
        ],
        "modified_luid": "0x0000000000002345",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "process_instance": {
        "pid": 1000,
        "created_filetime": "133000000000000000",
        "os_boot_id": "00000000-0000-4000-8000-000000000500"
      },
      "parent_process_instance": null,
      "sandbox_identity": {
        "mechanism": "restricted_token",
        "identity_id": "fixture-worker",
        "capability_sids": [],
        "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "agent_runtime_identity": {
        "product": "synthetic-worker",
        "version": "fixture-1",
        "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "generation": "00000000-0000-4000-8000-000000000501"
      }
    },
    "verifier_instance": "00000000-0000-4000-8000-000000000606",
    "verified_at": "2026-11-05T08:00:00Z"
  },
  "execution_mode": "B",
  "execution_state": "prepared",
  "policy_decision": "allow",
  "risk_level": "low",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": true,
    "single_use_grant_verified": true,
    "os_isolation_verified": false,
    "certificate": {
      "certificate_id": "00000000-0000-4000-8000-000000000901",
      "claim_scope_id": "00000000-0000-4000-8000-000000000900",
      "agent_id": "A01",
      "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "entrypoint_ids": [
        "fixture-read"
      ],
      "execution_mode": "B",
      "resource_scope": [
        "fixture-tree"
      ],
      "flags": {
        "telemetry_verified": false,
        "pre_execution_block_verified": false,
        "parameter_binding_verified": true,
        "single_use_grant_verified": true,
        "os_isolation_verified": false
      },
      "evidence_refs": [
        "00000000-0000-4000-8000-000000000902"
      ],
      "evidence_grade": "E3",
      "os_build": "synthetic-26200",
      "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "issued_at": "2026-11-05T08:00:00Z",
      "expires_at": "2026-11-05T09:00:00Z",
      "verified_by": "00000000-0000-4000-8000-000000000903",
      "fault_behavior": "verified_fail_closed",
      "effective_state": "valid"
    }
  },
  "grant": {
    "grant_id": "00000000-0000-4000-8000-000000000600",
    "state": "reserved",
    "previous_state": "pending",
    "reason": "synthetic",
    "reservation_id": "00000000-0000-4000-8000-000000000601",
    "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "key_id": "fixture-key",
    "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
    "policy_epoch": 1,
    "sequence": 3,
    "consumed_by_broker": null,
    "consumed_at": null
  },
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": null,
  "decision_id": "00000000-0000-4000-8000-000000000605",
  "source": {
    "kind": "core",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [],
  "missing_fields": [],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### NEG09

B启动前未消费。预期被对应Schema拒绝。

<!-- ag:fixture NEG09 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "execution.started",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": null,
  "target_paths": [],
  "network_requests": [],
  "principal_snapshot": {
    "status": "os_verified",
    "reported": null,
    "os_verified": {
      "windows_sid": "S-1-5-21-100-200-300-1001",
      "logon_luid": "0x0000000000001234",
      "integrity_level": "low",
      "token_restrictions": {
        "is_restricted": true,
        "restricting_sids": [
          "S-1-15-2-123"
        ],
        "deny_only_sids": [],
        "removed_privileges": [
          "SeDebugPrivilege"
        ],
        "modified_luid": "0x0000000000002345",
        "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "process_instance": {
        "pid": 1000,
        "created_filetime": "133000000000000000",
        "os_boot_id": "00000000-0000-4000-8000-000000000500"
      },
      "parent_process_instance": null,
      "sandbox_identity": {
        "mechanism": "restricted_token",
        "identity_id": "fixture-worker",
        "capability_sids": [],
        "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "agent_runtime_identity": {
        "product": "synthetic-worker",
        "version": "fixture-1",
        "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "generation": "00000000-0000-4000-8000-000000000501"
      }
    },
    "verifier_instance": "00000000-0000-4000-8000-000000000606",
    "verified_at": "2026-11-05T08:00:00Z"
  },
  "execution_mode": "B",
  "execution_state": "running",
  "policy_decision": null,
  "risk_level": "low",
  "capabilities": {
    "scope_id": "00000000-0000-4000-8000-000000000900",
    "level": "L0",
    "telemetry_verified": false,
    "pre_execution_block_verified": false,
    "parameter_binding_verified": true,
    "single_use_grant_verified": true,
    "os_isolation_verified": false,
    "certificate": {
      "certificate_id": "00000000-0000-4000-8000-000000000901",
      "claim_scope_id": "00000000-0000-4000-8000-000000000900",
      "agent_id": "A01",
      "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "entrypoint_ids": [
        "fixture-read"
      ],
      "execution_mode": "B",
      "resource_scope": [
        "fixture-tree"
      ],
      "flags": {
        "telemetry_verified": false,
        "pre_execution_block_verified": false,
        "parameter_binding_verified": true,
        "single_use_grant_verified": true,
        "os_isolation_verified": false
      },
      "evidence_refs": [
        "00000000-0000-4000-8000-000000000902"
      ],
      "evidence_grade": "E3",
      "os_build": "synthetic-26200",
      "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "issued_at": "2026-11-05T08:00:00Z",
      "expires_at": "2026-11-05T09:00:00Z",
      "verified_by": "00000000-0000-4000-8000-000000000903",
      "fault_behavior": "verified_fail_closed",
      "effective_state": "valid"
    }
  },
  "grant": {
    "grant_id": "00000000-0000-4000-8000-000000000600",
    "state": "reserved",
    "previous_state": "pending",
    "reason": "synthetic",
    "reservation_id": "00000000-0000-4000-8000-000000000601",
    "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "key_id": "fixture-key",
    "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
    "policy_epoch": 1,
    "sequence": 4,
    "consumed_by_broker": null,
    "consumed_at": null
  },
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "decision_id": "00000000-0000-4000-8000-000000000605",
  "source": {
    "kind": "broker",
    "instance_id": "00000000-0000-4000-8000-000000000003",
    "sequence": 1,
    "invocation_id": null,
    "authenticated": false,
    "source_event_id": "fixture-1"
  },
  "evidence": [],
  "missing_fields": [],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {}
}
~~~

### NEG10

网络观察缺网络对象。预期被对应Schema拒绝。

<!-- ag:fixture NEG10 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
  "agent_id": "A01",
  "agent_version": null,
  "session_id": null,
  "parent_session_id": null,
  "subagent_id": null,
  "parent_invocation_id": null,
  "depth": 0,
  "provider_id": null,
  "event_type": "network.observed",
  "timestamp": null,
  "received_at": "2026-11-05T08:00:00Z",
  "correlation_id": "00000000-0000-4000-8000-000000000002",
  "ingest_seq": 1,
  "tool_name": "fixture-read",
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
  "evidence": [],
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

### NEG11

Adapter伪造allow。预期被对应Schema拒绝。

<!-- ag:fixture NEG11 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
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
  "execution_state": "proposed",
  "policy_decision": "allow",
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
  "evidence": [],
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

### NEG12

无标志的L2。预期被对应Schema拒绝。

<!-- ag:fixture NEG12 capabilities-v2 invalid -->

~~~json
{
  "scope_id": "00000000-0000-4000-8000-000000000900",
  "level": "L2",
  "telemetry_verified": false,
  "pre_execution_block_verified": false,
  "parameter_binding_verified": false,
  "single_use_grant_verified": false,
  "os_isolation_verified": false,
  "certificate": null
}
~~~

### NEG13

隔离证据不足。预期被对应Schema拒绝。

<!-- ag:fixture NEG13 capabilities-v2 invalid -->

~~~json
{
  "scope_id": "00000000-0000-4000-8000-000000000900",
  "level": "L3",
  "telemetry_verified": false,
  "pre_execution_block_verified": false,
  "parameter_binding_verified": false,
  "single_use_grant_verified": false,
  "os_isolation_verified": true,
  "certificate": {
    "certificate_id": "00000000-0000-4000-8000-000000000901",
    "claim_scope_id": "00000000-0000-4000-8000-000000000900",
    "agent_id": "A01",
    "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "entrypoint_ids": [
      "fixture-read"
    ],
    "execution_mode": "C",
    "resource_scope": [
      "fixture-tree"
    ],
    "flags": {
      "telemetry_verified": false,
      "pre_execution_block_verified": false,
      "parameter_binding_verified": false,
      "single_use_grant_verified": false,
      "os_isolation_verified": true
    },
    "evidence_refs": [
      "00000000-0000-4000-8000-000000000902"
    ],
    "evidence_grade": "E3",
    "os_build": "synthetic-26200",
    "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "issued_at": "2026-11-05T08:00:00Z",
    "expires_at": "2026-11-05T09:00:00Z",
    "verified_by": "00000000-0000-4000-8000-000000000903",
    "fault_behavior": "verified_fail_closed",
    "effective_state": "valid"
  }
}
~~~

### NEG14

过期标志应false。预期被对应Schema拒绝。

<!-- ag:fixture NEG14 capabilities-v2 invalid -->

~~~json
{
  "scope_id": "00000000-0000-4000-8000-000000000900",
  "level": "L2",
  "telemetry_verified": false,
  "pre_execution_block_verified": true,
  "parameter_binding_verified": false,
  "single_use_grant_verified": false,
  "os_isolation_verified": false,
  "certificate": {
    "certificate_id": "00000000-0000-4000-8000-000000000901",
    "claim_scope_id": "00000000-0000-4000-8000-000000000900",
    "agent_id": "A01",
    "client_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "entrypoint_ids": [
      "fixture-read"
    ],
    "execution_mode": "A",
    "resource_scope": [
      "fixture-tree"
    ],
    "flags": {
      "telemetry_verified": false,
      "pre_execution_block_verified": true,
      "parameter_binding_verified": false,
      "single_use_grant_verified": false,
      "os_isolation_verified": false
    },
    "evidence_refs": [
      "00000000-0000-4000-8000-000000000902"
    ],
    "evidence_grade": "E3",
    "os_build": "synthetic-26200",
    "test_fixture_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "issued_at": "2026-11-05T08:00:00Z",
    "expires_at": "2026-11-05T09:00:00Z",
    "verified_by": "00000000-0000-4000-8000-000000000903",
    "fault_behavior": "known_fail_open",
    "effective_state": "expired"
  }
}
~~~

### NEG15

强绑定缺证书。预期被对应Schema拒绝。

<!-- ag:fixture NEG15 capabilities-v2 invalid -->

~~~json
{
  "scope_id": "00000000-0000-4000-8000-000000000900",
  "level": "L0",
  "telemetry_verified": false,
  "pre_execution_block_verified": false,
  "parameter_binding_verified": true,
  "single_use_grant_verified": false,
  "os_isolation_verified": false,
  "certificate": null
}
~~~

### NEG16

ticket只准一次。预期被对应Schema拒绝。

<!-- ag:fixture NEG16 permit-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "nonce": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "invocation_id": "00000000-0000-4000-8000-000000000702",
  "agent_id": "A01",
  "session_id": "00000000-0000-4000-8000-000000000701",
  "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "requestor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "executor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "policy_epoch": 1,
  "grant_epoch": 1,
  "expires_at": "2026-11-05T08:00:30Z",
  "ttl_ms": 30000,
  "max_uses": 2,
  "mac": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
~~~

### NEG17

TTL超限。预期被对应Schema拒绝。

<!-- ag:fixture NEG17 permit-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "nonce": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "invocation_id": "00000000-0000-4000-8000-000000000702",
  "agent_id": "A01",
  "session_id": "00000000-0000-4000-8000-000000000701",
  "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "requestor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "executor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "policy_epoch": 1,
  "grant_epoch": 1,
  "expires_at": "2026-11-05T08:00:30Z",
  "ttl_ms": 60001,
  "max_uses": 1,
  "mac": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
~~~

### NEG18

MAC长度错误。预期被对应Schema拒绝。

<!-- ag:fixture NEG18 permit-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "nonce": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "invocation_id": "00000000-0000-4000-8000-000000000702",
  "agent_id": "A01",
  "session_id": "00000000-0000-4000-8000-000000000701",
  "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "requestor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "executor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "policy_epoch": 1,
  "grant_epoch": 1,
  "expires_at": "2026-11-05T08:00:30Z",
  "ttl_ms": 30000,
  "max_uses": 1,
  "mac": "short"
}
~~~

### NEG19

typed参数闭合。预期被对应Schema拒绝。

<!-- ag:fixture NEG19 operation-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "agent_id": "A01",
  "runtime": {
    "product": "synthetic-worker",
    "version": "fixture-1",
    "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "generation": "00000000-0000-4000-8000-000000000501"
  },
  "adapter_id": "synthetic-adapter",
  "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "provider_id": "synthetic-provider",
  "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "session_id": "00000000-0000-4000-8000-000000000701",
  "invocation_id": "00000000-0000-4000-8000-000000000702",
  "requestor_principal": {
    "windows_sid": "S-1-5-21-100-200-300-1001",
    "logon_luid": "0x0000000000001234",
    "integrity_level": "low",
    "token_restrictions": {
      "is_restricted": true,
      "restricting_sids": [
        "S-1-15-2-123"
      ],
      "deny_only_sids": [],
      "removed_privileges": [
        "SeDebugPrivilege"
      ],
      "modified_luid": "0x0000000000002345",
      "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "process_instance": {
      "pid": 1000,
      "created_filetime": "133000000000000000",
      "os_boot_id": "00000000-0000-4000-8000-000000000500"
    },
    "parent_process_instance": null,
    "sandbox_identity": {
      "mechanism": "restricted_token",
      "identity_id": "fixture-worker",
      "capability_sids": [],
      "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "agent_runtime_identity": {
      "product": "synthetic-worker",
      "version": "fixture-1",
      "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "generation": "00000000-0000-4000-8000-000000000501"
    }
  },
  "executor_principal": {
    "windows_sid": "S-1-5-21-100-200-300-1001",
    "logon_luid": "0x0000000000001234",
    "integrity_level": "low",
    "token_restrictions": {
      "is_restricted": true,
      "restricting_sids": [
        "S-1-15-2-123"
      ],
      "deny_only_sids": [],
      "removed_privileges": [
        "SeDebugPrivilege"
      ],
      "modified_luid": "0x0000000000002345",
      "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "process_instance": {
      "pid": 1000,
      "created_filetime": "133000000000000000",
      "os_boot_id": "00000000-0000-4000-8000-000000000500"
    },
    "parent_process_instance": null,
    "sandbox_identity": {
      "mechanism": "restricted_token",
      "identity_id": "fixture-worker",
      "capability_sids": [],
      "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "agent_runtime_identity": {
      "product": "synthetic-worker",
      "version": "fixture-1",
      "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "generation": "00000000-0000-4000-8000-000000000501"
    }
  },
  "recipe": "file.read.v1",
  "command": null,
  "arguments": {
    "target_index": 0,
    "max_bytes": 4096,
    "shell": "evil"
  },
  "working_directory": {
    "path": "D:\\GuardLab",
    "volume_id": "fixture-volume",
    "file_id": "dir1",
    "content_digest": null,
    "kind": "directory",
    "verification": "handle_verified"
  },
  "targets": [
    {
      "path": "D:\\GuardLab\\example.txt",
      "volume_id": "fixture-volume",
      "file_id": "file1",
      "content_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "kind": "file",
      "verification": "handle_verified"
    }
  ],
  "executable_digest": null,
  "script_digest": null,
  "input_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "environment_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "stdin_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "native_permission": "allow",
  "network_authorization": null,
  "secret_parameters": {
    "key_id": "fixture-secret",
    "entries": []
  },
  "handle_manifest": [
    {
      "handle_ref": "00000000-0000-4000-8000-000000000703",
      "object_type": "file",
      "object_identity_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "granted_access": 1179785,
      "inherit_to_worker": false,
      "purpose": "target"
    }
  ]
}
~~~

### NEG20

缺原生许可。预期被对应Schema拒绝。

<!-- ag:fixture NEG20 operation-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "agent_id": "A01",
  "runtime": {
    "product": "synthetic-worker",
    "version": "fixture-1",
    "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "generation": "00000000-0000-4000-8000-000000000501"
  },
  "adapter_id": "synthetic-adapter",
  "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "provider_id": "synthetic-provider",
  "provider_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "config_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "session_id": "00000000-0000-4000-8000-000000000701",
  "invocation_id": "00000000-0000-4000-8000-000000000702",
  "requestor_principal": {
    "windows_sid": "S-1-5-21-100-200-300-1001",
    "logon_luid": "0x0000000000001234",
    "integrity_level": "low",
    "token_restrictions": {
      "is_restricted": true,
      "restricting_sids": [
        "S-1-15-2-123"
      ],
      "deny_only_sids": [],
      "removed_privileges": [
        "SeDebugPrivilege"
      ],
      "modified_luid": "0x0000000000002345",
      "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "process_instance": {
      "pid": 1000,
      "created_filetime": "133000000000000000",
      "os_boot_id": "00000000-0000-4000-8000-000000000500"
    },
    "parent_process_instance": null,
    "sandbox_identity": {
      "mechanism": "restricted_token",
      "identity_id": "fixture-worker",
      "capability_sids": [],
      "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "agent_runtime_identity": {
      "product": "synthetic-worker",
      "version": "fixture-1",
      "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "generation": "00000000-0000-4000-8000-000000000501"
    }
  },
  "executor_principal": {
    "windows_sid": "S-1-5-21-100-200-300-1001",
    "logon_luid": "0x0000000000001234",
    "integrity_level": "low",
    "token_restrictions": {
      "is_restricted": true,
      "restricting_sids": [
        "S-1-15-2-123"
      ],
      "deny_only_sids": [],
      "removed_privileges": [
        "SeDebugPrivilege"
      ],
      "modified_luid": "0x0000000000002345",
      "token_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "process_instance": {
      "pid": 1000,
      "created_filetime": "133000000000000000",
      "os_boot_id": "00000000-0000-4000-8000-000000000500"
    },
    "parent_process_instance": null,
    "sandbox_identity": {
      "mechanism": "restricted_token",
      "identity_id": "fixture-worker",
      "capability_sids": [],
      "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    },
    "agent_runtime_identity": {
      "product": "synthetic-worker",
      "version": "fixture-1",
      "binary_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "generation": "00000000-0000-4000-8000-000000000501"
    }
  },
  "recipe": "file.read.v1",
  "command": null,
  "arguments": {
    "target_index": 0,
    "max_bytes": 4096
  },
  "working_directory": {
    "path": "D:\\GuardLab",
    "volume_id": "fixture-volume",
    "file_id": "dir1",
    "content_digest": null,
    "kind": "directory",
    "verification": "handle_verified"
  },
  "targets": [
    {
      "path": "D:\\GuardLab\\example.txt",
      "volume_id": "fixture-volume",
      "file_id": "file1",
      "content_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "kind": "file",
      "verification": "handle_verified"
    }
  ],
  "executable_digest": null,
  "script_digest": null,
  "input_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "environment_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "stdin_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "native_permission": "unknown",
  "network_authorization": null,
  "secret_parameters": {
    "key_id": "fixture-secret",
    "entries": []
  },
  "handle_manifest": [
    {
      "handle_ref": "00000000-0000-4000-8000-000000000703",
      "object_type": "file",
      "object_identity_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "granted_access": 1179785,
      "inherit_to_worker": false,
      "purpose": "target"
    }
  ]
}
~~~

### NEG21

网络授权缺IP。预期被对应Schema拒绝。

<!-- ag:fixture NEG21 network-grant-v2 invalid -->

~~~json
{
  "authorization_id": "00000000-0000-4000-8000-000000000801",
  "scope_id": "00000000-0000-4000-8000-000000000802",
  "request_method": "GET",
  "scheme": "https",
  "host": "sink.invalid",
  "port": 443,
  "destination_ip": null,
  "dns_answers": [
    {
      "ip": "192.0.2.1",
      "family": 4
    }
  ],
  "dns_epoch": "00000000-0000-4000-8000-000000000803",
  "dns_expires_at": "2026-11-05T08:00:30Z",
  "resolver_id": "fixture-resolver",
  "redirect_chain": [],
  "redirect_limit": 0,
  "proxy": {
    "mode": "broker",
    "proxy_id": "fixture-network",
    "endpoint_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  },
  "transport": "tcp",
  "path_query_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "upload_data_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "request_body_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "max_request_bytes": 1024,
  "max_response_bytes": 4096,
  "max_requests": 1,
  "connection_id": "00000000-0000-4000-8000-000000000804",
  "pool_scope": "00000000-0000-4000-8000-000000000805",
  "connection_policy_epoch": 1,
  "observation_state": "request_seen",
  "enforcement_state": "broker_egress",
  "enforcement_scope_id": "00000000-0000-4000-8000-000000000802",
  "native_permission": "allow"
}
~~~

### NEG22

首版recipe无QUIC。预期被对应Schema拒绝。

<!-- ag:fixture NEG22 network-grant-v2 invalid -->

~~~json
{
  "authorization_id": "00000000-0000-4000-8000-000000000801",
  "scope_id": "00000000-0000-4000-8000-000000000802",
  "request_method": "GET",
  "scheme": "https",
  "host": "sink.invalid",
  "port": 443,
  "destination_ip": "192.0.2.1",
  "dns_answers": [
    {
      "ip": "192.0.2.1",
      "family": 4
    }
  ],
  "dns_epoch": "00000000-0000-4000-8000-000000000803",
  "dns_expires_at": "2026-11-05T08:00:30Z",
  "resolver_id": "fixture-resolver",
  "redirect_chain": [],
  "redirect_limit": 0,
  "proxy": {
    "mode": "broker",
    "proxy_id": "fixture-network",
    "endpoint_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  },
  "transport": "quic",
  "path_query_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "upload_data_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "request_body_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "max_request_bytes": 1024,
  "max_response_bytes": 4096,
  "max_requests": 1,
  "connection_id": "00000000-0000-4000-8000-000000000804",
  "pool_scope": "00000000-0000-4000-8000-000000000805",
  "connection_policy_epoch": 1,
  "observation_state": "request_seen",
  "enforcement_state": "broker_egress",
  "enforcement_scope_id": "00000000-0000-4000-8000-000000000802",
  "native_permission": "allow"
}
~~~

### NEG23

未知正文不等于空。预期被对应Schema拒绝。

<!-- ag:fixture NEG23 network-grant-v2 invalid -->

~~~json
{
  "authorization_id": "00000000-0000-4000-8000-000000000801",
  "scope_id": "00000000-0000-4000-8000-000000000802",
  "request_method": "GET",
  "scheme": "https",
  "host": "sink.invalid",
  "port": 443,
  "destination_ip": "192.0.2.1",
  "dns_answers": [
    {
      "ip": "192.0.2.1",
      "family": 4
    }
  ],
  "dns_epoch": "00000000-0000-4000-8000-000000000803",
  "dns_expires_at": "2026-11-05T08:00:30Z",
  "resolver_id": "fixture-resolver",
  "redirect_chain": [],
  "redirect_limit": 0,
  "proxy": {
    "mode": "broker",
    "proxy_id": "fixture-network",
    "endpoint_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  },
  "transport": "tcp",
  "path_query_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "upload_data_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "request_body_digest": null,
  "max_request_bytes": 1024,
  "max_response_bytes": 4096,
  "max_requests": 1,
  "connection_id": "00000000-0000-4000-8000-000000000804",
  "pool_scope": "00000000-0000-4000-8000-000000000805",
  "connection_policy_epoch": 1,
  "observation_state": "request_seen",
  "enforcement_state": "broker_egress",
  "enforcement_scope_id": "00000000-0000-4000-8000-000000000802",
  "native_permission": "allow"
}
~~~

### NEG24

代理不等于隔离出口。预期被对应Schema拒绝。

<!-- ag:fixture NEG24 network-grant-v2 invalid -->

~~~json
{
  "authorization_id": "00000000-0000-4000-8000-000000000801",
  "scope_id": "00000000-0000-4000-8000-000000000802",
  "request_method": "GET",
  "scheme": "https",
  "host": "sink.invalid",
  "port": 443,
  "destination_ip": "192.0.2.1",
  "dns_answers": [
    {
      "ip": "192.0.2.1",
      "family": 4
    }
  ],
  "dns_epoch": "00000000-0000-4000-8000-000000000803",
  "dns_expires_at": "2026-11-05T08:00:30Z",
  "resolver_id": "fixture-resolver",
  "redirect_chain": [],
  "redirect_limit": 0,
  "proxy": {
    "mode": "broker",
    "proxy_id": "fixture-network",
    "endpoint_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  },
  "transport": "tcp",
  "path_query_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "upload_data_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "request_body_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "max_request_bytes": 1024,
  "max_response_bytes": 4096,
  "max_requests": 1,
  "connection_id": "00000000-0000-4000-8000-000000000804",
  "pool_scope": "00000000-0000-4000-8000-000000000805",
  "connection_policy_epoch": 1,
  "observation_state": "request_seen",
  "enforcement_state": "request_proxy",
  "enforcement_scope_id": "00000000-0000-4000-8000-000000000802",
  "native_permission": "allow"
}
~~~

### NEG25

非法created直接consumed。预期被对应Schema拒绝。

<!-- ag:fixture NEG25 grant-v2 invalid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "consumed",
  "previous_state": "created",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 4,
  "consumed_by_broker": "00000000-0000-4000-8000-000000000603",
  "consumed_at": "2026-11-05T08:00:00Z"
}
~~~

### NEG26

consumed不能回reserved。预期被对应Schema拒绝。

<!-- ag:fixture NEG26 grant-v2 invalid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "reserved",
  "previous_state": "consumed",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": null,
  "consumed_at": null
}
~~~

### BD01

整数、空值、数组与诊断字符串上界。预期通过对应Schema。

<!-- ag:fixture BD01 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": {
    "safe_integer": 9007199254740991,
    "empty": "",
    "nil": null,
    "ordered": [
      2,
      1
    ]
  },
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
  "evidence": [],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {
    "diagnostic.note": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
  }
}
~~~

### NEG27

不安全整数。预期被对应Schema拒绝。

<!-- ag:fixture NEG27 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": {
    "safe_integer": 9007199254740992,
    "empty": "",
    "nil": null,
    "ordered": [
      2,
      1
    ]
  },
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
  "evidence": [],
  "missing_fields": [
    "principal_snapshot.os_verified"
  ],
  "redaction": {
    "profile": "audit-safe-v2",
    "fields": [],
    "raw_retained": false
  },
  "extensions": {
    "diagnostic.note": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
  }
}
~~~

### BD02

TTL上界。预期通过对应Schema。

<!-- ag:fixture BD02 permit-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "nonce": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "broker_instance": "00000000-0000-4000-8000-000000000603",
  "prepared_id": "00000000-0000-4000-8000-000000000604",
  "invocation_id": "00000000-0000-4000-8000-000000000702",
  "agent_id": "A01",
  "session_id": "00000000-0000-4000-8000-000000000701",
  "runtime_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "adapter_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "requestor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "executor_principal_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "semantic_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "policy_epoch": 1,
  "grant_epoch": 1,
  "expires_at": "2026-11-05T08:00:30Z",
  "ttl_ms": 60000,
  "max_uses": 1,
  "mac": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
~~~

### BD03

路径长度上界。预期通过对应Schema。

<!-- ag:fixture BD03 event-v2 valid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
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
  "evidence": [],
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

### NEG28

路径超长。预期被对应Schema拒绝。

<!-- ag:fixture NEG28 event-v2 invalid -->

~~~json
{
  "schema_version": "2.0.0",
  "record_kind": "synthetic_fixture",
  "event_id": "00000000-0000-4000-8000-000000000001",
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
  "tool_name": "fixture-read",
  "command": null,
  "arguments": null,
  "working_directory": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
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
  "evidence": [],
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

### GR-created

生命周期合法转换合成例。预期通过对应Schema。

<!-- ag:fixture GR-created grant-v2 valid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "created",
  "previous_state": null,
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": null,
  "consumed_at": null
}
~~~

### GR-pending

生命周期合法转换合成例。预期通过对应Schema。

<!-- ag:fixture GR-pending grant-v2 valid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "pending",
  "previous_state": "created",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": null,
  "consumed_at": null
}
~~~

### GR-reserved

生命周期合法转换合成例。预期通过对应Schema。

<!-- ag:fixture GR-reserved grant-v2 valid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "reserved",
  "previous_state": "pending",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": null,
  "consumed_at": null
}
~~~

### GR-consumed

生命周期合法转换合成例。预期通过对应Schema。

<!-- ag:fixture GR-consumed grant-v2 valid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "consumed",
  "previous_state": "reserved",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": "00000000-0000-4000-8000-000000000603",
  "consumed_at": "2026-11-05T08:00:00Z"
}
~~~

### GR-revoked

生命周期合法转换合成例。预期通过对应Schema。

<!-- ag:fixture GR-revoked grant-v2 valid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "revoked",
  "previous_state": "reserved",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": null,
  "consumed_at": null
}
~~~

### GR-expired

生命周期合法转换合成例。预期通过对应Schema。

<!-- ag:fixture GR-expired grant-v2 valid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "expired",
  "previous_state": "reserved",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": null,
  "consumed_at": null
}
~~~

### GR-invalidated

生命周期合法转换合成例。预期通过对应Schema。

<!-- ag:fixture GR-invalidated grant-v2 valid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "invalidated",
  "previous_state": "reserved",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": null,
  "consumed_at": null
}
~~~

### GR-failed

生命周期合法转换合成例。预期通过对应Schema。

<!-- ag:fixture GR-failed grant-v2 valid -->

~~~json
{
  "grant_id": "00000000-0000-4000-8000-000000000600",
  "state": "failed",
  "previous_state": "reserved",
  "reason": "synthetic",
  "reservation_id": "00000000-0000-4000-8000-000000000601",
  "operation_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "nonce_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "key_id": "fixture-key",
  "core_boot_epoch": "00000000-0000-4000-8000-000000000602",
  "policy_epoch": 1,
  "sequence": 3,
  "consumed_by_broker": null,
  "consumed_at": null
}
~~~

### CP00

七款实际初始能力模板，scope为合成ID。预期通过对应Schema。

<!-- ag:fixture CP00 capabilities-v2 valid -->

~~~json
{
  "scope_id": "00000000-0000-4000-8000-000000000900",
  "level": "L0",
  "telemetry_verified": false,
  "pre_execution_block_verified": false,
  "parameter_binding_verified": false,
  "single_use_grant_verified": false,
  "os_isolation_verified": false,
  "certificate": null
}
~~~

## 关联

[正式协议](07-UNIFIED-EVENT-SCHEMA.md) [向量和编码](18-CRYPTOGRAPHIC-CONTRACT.md) [测试计划](10-TEST-AND-ACCEPTANCE-PLAN.md) [P1任务](14-P1-DEVELOPMENT-PLAN.md)

