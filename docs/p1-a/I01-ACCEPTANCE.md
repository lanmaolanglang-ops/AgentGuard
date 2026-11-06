# I01节点验收

AgentGuard V2.0 · P1-A · 2026-11-06

## 结论

PASS，允许启动I02-A、I20-A与I23。身份未知以显式缺口交付，七款全部L0、五标志false，无真实Agent运行认证。

## 实际文件

- [.gitignore](../../.gitignore)
- [.gitattributes](../../.gitattributes)
- [docs/p1-a/evidence/I01/design-baseline.json](evidence/I01/design-baseline.json)
- [docs/p1-a/evidence/I01/agent-inventory.json](evidence/I01/agent-inventory.json)
- [docs/p1-a/evidence/I01/environment.json](evidence/I01/environment.json)
- [tools/baseline.py](../../tools/baseline.py)
- [tests/python/test_baseline.py](../../tests/python/test_baseline.py)
- [docs/p1-a/SCOPE-AND-ACCEPTANCE.md](SCOPE-AND-ACCEPTANCE.md)
- [README.md](../../README.md)
- [docs/p1-a/PROGRESS.md](PROGRESS.md)

I01验收时已初始化本地Git，尚未设置远端。负责人随后提供GitHub地址，本批交付基于origin/main建立独立分支p1-a/first-batch并推送。P0或P0.1字节保持冻结。

## 架构与依赖

只增加开发校验入口与身份/环境数据，核心职责和安全边界保持P0.1合同。Python stdlib实现基线与8个回归测试，无第三方依赖。Rust 1.97.1与MSVC BuildTools 2022、Node 24.19.0、Python 3.14.6已存在。cargo/rustc路径摘要对应rustup shim，环境记录明确这一局限。

当前构建对象是Python校验工具，运行入口为python -B tools/baseline.py，测试入口为python -B -m unittest discover -s tests/python -v。I03 Rust入口预定cargo build --workspace --locked与cargo test --workspace --locked，独立Core仅处理合成协议输入。UI与SQLite业务模块按后续节点交付。

## 测试记录

执行python -B tools/baseline.py，通过42份设计字节哈希、7款L0清单和24父任务/27节点/117依赖边、图表相同与DAG无环检查。执行Python unittest共8项，PASS 8、FAIL 0，含篡改manifest、能力true、证据升级、额外Agent、WorkBuddy混淆、缺字段和依赖图变更反例。

## 安全影响与未覆盖

本轮读取有限卸载/Appx元数据和四个选定PE的hash/Authenticode，不启动Agent或读取其配置/会话。Reasonix只有卸载版本，Claude/豆包客户端版本未知，ZCode PE给出Electron 41.0.3而非产品版本且NotSigned，均保留缺口。签名Valid仅针对选定PE，未完成整包/内核版本映射。

对应Issue I01，支持AT20/24/34的纯数据基线控制。没有执行这些AT的真实Agent变体。无UI、运行授权、网络隔离或危险测试，不产生接入/保护承诺。

[下一节点进度](PROGRESS.md)
