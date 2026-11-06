# I20-A早期供应链基线

AgentGuard V2.0 · 2026-11-06

本节点仅固定I03必要依赖及开发工具，最终发布复核I20-C保持后置。Rust源代码在I03前置验收通过后加入，当前只登记manifest和lock。

## 版本与来源

Rust工具链目标1.97.1 x86_64-pc-windows-msvc，现有stable别名实际为1.97.1。CI将精确安装1.97.1，本地命令记录实际版本并由CI脚本核对。原生FFI仅通过windows-sys 0.61.2封装于Core的独立Windows模块。

Cargo直接依赖全部精确版本，transitive由Cargo.lock固定。jsonschema 0.60.2关闭所有default features，禁用resolve-http/resolve-file/TLS/IDNA后端，仅本地URN资源。RustCrypto hmac 0.12.1与sha2 0.10.9执行固定向量，生产密钥和ledger属于I06/I05。

Python固定jsonschema 4.26.0及闭包，官方PyPI wheel SHA-256存入requirements-ci.lock与证据。补齐rfc3339-validator/six，以确保date-time严格启用。安装限定项目.venv、--require-hashes、--only-binary=:all:，不运行sdist构建脚本。

## 审查范围

cargo info已核对jsonschema/Tokio/RustCrypto/windows-rs官方仓库、license和minimum Rust。registry依赖解析后以Cargo.lock校验和与cargo metadata生成开发SBOM，检查未知源及build.rs。开发依赖源码审查聚焦feature/网络retriever、密码学verify_slice、Windows FFI边界和构建脚本，不能称完整供应链安全认证。

当前没有SQLite实现或SQLite运行依赖。后续I05选型必须核对明确bundled版本、WAL已知缺陷与durability回归，I20-C最终审查仍待进行。Tauri/React/TypeScript属于I09，本批不引入UI依赖或改变技术栈。

## 风险与未覆盖

新crate具有解析器与传递依赖攻击面。锁版本和hash验证只能固定内容，不能证明没有逻辑漏洞。依赖漏洞数据库查询、构建脚本清单和SBOM在本节点结果记录，任何已知高危影响需停止相关构建并处理。不存在真实Agent探针或生产配置变更。
