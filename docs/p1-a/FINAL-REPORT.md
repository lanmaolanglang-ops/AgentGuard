# P1-A第一批交付报告

AgentGuard V2.0 · 2026-11-06

## 1 已完成节点

I01身份/工程基线、I02-A合成夹具、I20-A早期供应链、I23来源复核已PASS。I03本地实现与统一10门禁已通过，GitHub CI真实运行结果待记录后授予最终PASS。

## 2 未完成节点

本批范围之外I04/I05/I06/I08/I09/I22及所有P1-B/P1-C任务保持后置。完整Prepare→Attest→Approval→Reserve→Redeem→Execute→Evidence依赖I04/I05/I06等实现，当前接口统一不发行执行许可。D01–D05全部待负责人决定。

## 3 实际结构

~~~text
crates/protocol/src    严格parser、标准Schema、typed HMAC、导入迁移、受限ingress
crates/core/src        独立Core、合成probe、有界IPC和Windows只读身份FFI
protocol/schemas      冻结9份Schema的可验证提取副本
fixtures              65夹具与未物化虚拟文件树
requirements-ci.lock  Python wheel/hash依赖锁
Cargo.lock            Rust依赖版本/checksum锁
.github/workflows     固定版本Windows CI工作流
tests/python          17项基线、夹具、来源和供应链回归
tools                 同入口CI、来源/供应链复核与13项真实合成进程/CLI测试
docs/p1-a             节点验收、证据、进度和本报告
~~~

## 4 技术选型与实现判断

延续独立Rust Core、Named Pipes与标准Draft2020-12，不提前引入I09 UI或I05 SQLite运行模块。包版本0.1.0表示工程接口初始版本，项目设计版本仍为AgentGuard V2.0。内部IPC采用u32 big-endian帧、5秒deadline、16请求连接预算，均为现有有界IPC要求的具体实现参数，未修改冻结Schema或能力语义。

Python只负责开发验证，安全协议/解析与MAC在Rust实现。生产密钥/nonce账本/最终Broker尚未实现。只读身份并不构成T2-B或OS隔离认证。公开证据JSON将用户目录前缀表示为%USERPROFILE%，保持所选PE hash/签名与未知信息。

## 5 测试报告

本地10门禁全部通过。Python 17、Rust 14、实际合成进程/CLI 13，共44个测试函数，FAIL 0。展开覆盖65结构夹具、21跨语言typed/frame/MAC向量、8旧版迁移与64状态对。测试通过数与数据用例展开数分别记录。

构建、Rustfmt、Clippy -D warnings均通过。真实IPC检验操作系统peer PID/实例/token/image、显式logon SID DACL、管道抢占、拒绝伪造allow/OS身份、重放、截断、超限、timeout与CLI读取限额。

## 6 安全审查

核心入口没有任意命令执行、权限签发、生产代理、HANDLE整数解引用或真实Agent适配。客户端受保护字段被拒绝，source冲突不合并，错误消息不回显参数。FFI集中且OwnedHandle/LocalFree生命周期明确，Token缓冲区在SID读取时保持存活。

跨用户/IL、remote-client实际负测、真Host关联、持久grant并发与崩溃恢复、Windows资源边界和独立Observer均未认证。声明范围保持I03合成协议与同上下文进程测试，七款L0和五false。

## 7 新问题

NI01补齐date-time format依赖，NI02更换命中公告的Tokio候选版本，NI03记录ZCode PE身份/签名缺口，NI04保留Reasonix固定文档内部冲突。均见新问题登记，没有修改P0.1或擅自关闭重大决策。

## 8 下一批依赖

I03最终PASS后I04/I05可以按图启动，I08还需I05，I06需I04/I05，I09需I06/I08，I22需I05/I06。I02-B仍需专用隔离方案与实际权限授权，本批不进入。

## 9 负责人事项

D01–D05待决，分别关联安全主体、网络backend/部署、GUI兼容、真实委托recipe/产品与密钥/反回退锚。它们不阻止当前纯合成协议完成，阻止相应真实保护声明。

## 10 产品方向与交付状态

中文错误说明身份/内容变化与许可缺失，审批/结构通过/真实执行明确分开。未实现无关功能，七款名单与状态固定。完成本批并提交可审查分支后停止，等待下一批任务。

[进度](PROGRESS.md) [I03验收](I03-ACCEPTANCE.md) [实际变更](CHANGED-FILES.md) [新问题](NEW-FINDINGS.md) [本地门禁](evidence/I03/local-gates.json)
