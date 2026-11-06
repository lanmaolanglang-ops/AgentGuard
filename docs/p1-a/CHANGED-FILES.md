# P1-A第一批实际变更索引

AgentGuard V2.0 · 2026-11-06

原docs/p0的15份与docs/p0.1的27份只纳入Git管理，字节未修改。target/.venv/.work不纳入提交。

## I01

- [.gitattributes](../../.gitattributes)
- [.gitignore](../../.gitignore)
- [README.md](../../README.md)
- [tools/baseline.py](../../tools/baseline.py)
- [tests/python/test_baseline.py](../../tests/python/test_baseline.py)

## I02-A

- [tools/extract_protocol.py](../../tools/extract_protocol.py)
- [tools/check_fixtures.py](../../tools/check_fixtures.py)
- [tools/__init__.py](../../tools/__init__.py)
- [tests/python/test_fixtures.py](../../tests/python/test_fixtures.py)
- [protocol/schemas/capabilities-v2.json](../../protocol/schemas/capabilities-v2.json)
- [protocol/schemas/crypto-vectors-v2.json](../../protocol/schemas/crypto-vectors-v2.json)
- [protocol/schemas/event-v1.json](../../protocol/schemas/event-v1.json)
- [protocol/schemas/event-v2.json](../../protocol/schemas/event-v2.json)
- [protocol/schemas/grant-v2.json](../../protocol/schemas/grant-v2.json)
- [protocol/schemas/network-grant-v2.json](../../protocol/schemas/network-grant-v2.json)
- [protocol/schemas/operation-v2.json](../../protocol/schemas/operation-v2.json)
- [protocol/schemas/permit-v2.json](../../protocol/schemas/permit-v2.json)
- [protocol/schemas/types-v2.json](../../protocol/schemas/types-v2.json)
- [fixtures/p0.1.json](../../fixtures/p0.1.json)
- [fixtures/virtual-tree.json](../../fixtures/virtual-tree.json)

## I20-A

- [Cargo.toml](../../Cargo.toml)
- [Cargo.lock](../../Cargo.lock)
- [requirements-ci.lock](../../requirements-ci.lock)
- [crates/protocol/Cargo.toml](../../crates/protocol/Cargo.toml)
- [crates/core/Cargo.toml](../../crates/core/Cargo.toml)
- [tools/review_supply_chain.py](../../tools/review_supply_chain.py)

## I23

- [tools/review_sources.py](../../tools/review_sources.py)
- [tools/review_conflicts.py](../../tools/review_conflicts.py)
- [tests/python/test_supply_and_sources.py](../../tests/python/test_supply_and_sources.py)

## I03

- [tools/check_crypto.py](../../tools/check_crypto.py)
- [tools/ci.py](../../tools/ci.py)
- [tools/test_ipc.py](../../tools/test_ipc.py)
- [.github/workflows/p1-a.yml](../../.github/workflows/p1-a.yml)
- [crates/core/src/ipc.rs](../../crates/core/src/ipc.rs)
- [crates/core/src/lib.rs](../../crates/core/src/lib.rs)
- [crates/core/src/main.rs](../../crates/core/src/main.rs)
- [crates/core/src/probe.rs](../../crates/core/src/probe.rs)
- [crates/core/src/windows.rs](../../crates/core/src/windows.rs)
- [crates/protocol/src/crypto.rs](../../crates/protocol/src/crypto.rs)
- [crates/protocol/src/ingress.rs](../../crates/protocol/src/ingress.rs)
- [crates/protocol/src/lib.rs](../../crates/protocol/src/lib.rs)
- [crates/protocol/src/migration.rs](../../crates/protocol/src/migration.rs)
- [crates/protocol/src/parse.rs](../../crates/protocol/src/parse.rs)
- [crates/protocol/src/schema.rs](../../crates/protocol/src/schema.rs)
- [crates/protocol/tests/contracts.rs](../../crates/protocol/tests/contracts.rs)

## 进度与证据

- [evidence/I01/agent-inventory.json](evidence/I01/agent-inventory.json)
- [evidence/I01/design-baseline.json](evidence/I01/design-baseline.json)
- [evidence/I01/environment.json](evidence/I01/environment.json)
- [evidence/I02-A/frozen-reference-results.json](evidence/I02-A/frozen-reference-results.json)
- [evidence/I02-A/schema-results.json](evidence/I02-A/schema-results.json)
- [evidence/I03/local-gates.json](evidence/I03/local-gates.json)
- [evidence/I20-A/build-script-review.json](evidence/I20-A/build-script-review.json)
- [evidence/I20-A/development-sbom.json](evidence/I20-A/development-sbom.json)
- [evidence/I20-A/python-package-sources.json](evidence/I20-A/python-package-sources.json)
- [evidence/I20-A/RUSTSEC-2026-0386.json](evidence/I20-A/RUSTSEC-2026-0386.json)
- [evidence/I23/fixed-source-excerpts.json](evidence/I23/fixed-source-excerpts.json)
- [evidence/I23/source-reachability.json](evidence/I23/source-reachability.json)
- [I01-ACCEPTANCE.md](I01-ACCEPTANCE.md)
- [I02-A-ACCEPTANCE.md](I02-A-ACCEPTANCE.md)
- [I03-ACCEPTANCE.md](I03-ACCEPTANCE.md)
- [I03-IPC-CONTRACT.md](I03-IPC-CONTRACT.md)
- [I20-A-ACCEPTANCE.md](I20-A-ACCEPTANCE.md)
- [I20-A-SUPPLY-CHAIN.md](I20-A-SUPPLY-CHAIN.md)
- [I23-ACCEPTANCE.md](I23-ACCEPTANCE.md)
- [NEW-FINDINGS.md](NEW-FINDINGS.md)
- [PROGRESS.md](PROGRESS.md)
- [SCOPE-AND-ACCEPTANCE.md](SCOPE-AND-ACCEPTANCE.md)

本机用户目录前缀在可公开的身份/环境JSON中表示为%USERPROFILE%，binary hash与签名查询结果保留。开发工具记录的rustup shim hash及其局限也保留。
