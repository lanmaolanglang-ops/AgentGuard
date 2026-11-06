# I20-A节点验收

AgentGuard V2.0 · P1-A · 2026-11-06

## 结论与依赖

PASS，I01已通过。完成早期依赖来源/版本/锁定和构建脚本风险基线，允许I03按冻结协议开始代码实现。I20-C最终发布审查保持未执行。

## 实际修改文件

Cargo.toml、Cargo.lock、crates/protocol/Cargo.toml、crates/core/Cargo.toml、requirements-ci.lock、tools/review_supply_chain.py、I20-A-SUPPLY-CHAIN.md、本报告与evidence/I20-A下Python来源、开发SBOM、构建脚本审查及RUSTSEC证据。当前manifest只声明后续I03源文件入口，尚未进行Rust业务构建。

## 架构、版本与安全影响

Rust Core保持独立，标准validator jsonschema 0.60.2关闭default网络/文件/TLS features，RustCrypto hmac 0.12.1/sha2 0.10.9、serde/serde_json、uuid、Tokio 1.54.1与windows-sys 0.61.2固定版本。Cargo.lock固定104个registry包，全部crates.io source及checksum存在。Python锁7包及官方PyPI wheel hash，限定.venv、wheel与require-hashes，安装后pip check通过。

## 发现与处理

OSV初查Tokio缓存候选1.53.1命中RUSTSEC-2026-0386，影响Unix AsyncFd I/O安全。直接选用已修补1.54.1并重新锁定。复查Rust104+Python7共111个版本查询，返回已知advisory 0条。该结果只代表本次数据库可见性，不构成完整安全认证。

21个包声明build.rs，已登记hash并检查构建副作用，主要为Rust编译器能力探测和OUT_DIR生成，部分WASM/Unix脚本不属于Windows构建路径。当前无SQLite/数据库运行依赖，I05仍需核对选定SQLite版本与WAL缺陷。外部工具与构建产物没有获得Agent执行权。

## 测试与未覆盖

供应链自动回归2项通过、0项失败，分别检查锁定开发SBOM/公告状态与Python wheel来源/hash。完整测试记录由统一门禁的Python测试日志提供。

Cargo锁生成/metadata来源与checksum检查通过，SBOM依赖清单104包、OSV 111查询无剩余命中，Python锁安装与完整性检查通过。失败路径中旧Tokio已更换，日期format缺包已补齐。对应Issue I20-A，支持AT23/29/34/46的开发供应链前提，真实AT未执行。代码审查、漏洞零日、最终签名/升级/回滚、发布性能属于后续节点。

[供应链方案](I20-A-SUPPLY-CHAIN.md) [开发SBOM](evidence/I20-A/development-sbom.json) [构建脚本](evidence/I20-A/build-script-review.json)
