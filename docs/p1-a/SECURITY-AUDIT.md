# P1-A第一批安全自审

AgentGuard V2.0 · 2026-11-06 · 对应代码9e8a728

## 审查范围与结果

开发者自审覆盖12个自有Rust源文件、开发脚本、依赖锁、合成夹具和CI权限。结论为本批合成工程PASS。代码9e8a728的真实Windows CI运行42152532076已成功，10门禁及44测试通过、失败0，见[CI回执](evidence/I03/github-ci.json)。独立发布安全复核I20-C仍待后续。

| 项目 | 实现与检查 | 当前边界 |
|---|---|---|
| JSON解析 | 字节上限、深度8、重复键、数字词法、Unicode及集合限额在Schema前检查 | 不处理任意用户命令语义 |
| 正式结构 | 9个冻结Schema、标准Draft2020-12、format强制、只读URN资源map | Schema合法不产生执行许可 |
| 字段所有权 | 客户端闭合Observation，nullable字段必须显式提供，protected字段拒绝 | 原生Adapter及Host血缘未接入 |
| MAC | 六固定域、typed编码、严格32字节解码、RustCrypto verify_slice | 仅公开测试key，无生产key存储/授权签票服务 |
| IPC身份 | OS peer PID、创建时间、SID/logon/IL/token、image pin和每请求重核 | 相同SID注入/替换进程仍为D01残余范围，image pin不等同加载映像证明 |
| IPC对象 | 显式logon SID DACL、remote reject、first-instance、非继承句柄 | 其他用户/IL和remote实际负测未进行 |
| IPC预算 | 1MiB帧、5秒deadline、每连接16请求、source冲突拒绝 | 当前仅单连接合成校验进程 |
| 句柄与FFI | 真实pipe类型进入私有HANDLE封装，OwnedHandle/LocalFree RAII，token缓冲区显式保持存活 | 没有Broker资源句柄传递或隔离认证 |
| 文件读取 | CLI限1MiB+1，PE hash限256MiB+1，均只读 | 未实现任意文件访问保护或全量读取观察 |
| 导入隐私 | 只接受8个冻结样本canonical hash，改入假秘密会拒绝且不回显 | 真实日志需I08脱敏后开放 |
| 错误输出 | 固定中文code/message，无原始命令/参数回显 | 完整自然语言解释与UI属I08/I09 |
| 依赖 | 104 Rust+7 Python锁定，OSV查询111版本，旧Tokio已换为1.54.1 | 不构成零日或完整供应链安全保证 |
| CI权限 | Actions固定SHA、contents read、只上传.work/ci合成测试日志 | 首次YAML错误已修复，真实运行结果保留 |

## 执行与授权隔离

自有Rust代码没有Command::new、CreateProcess、注册表写入、Firewall/WFP管理接口。独立合成probe只向本批Core专用管道发数据。Python进程测试只创建/回收本项目Core/probe及项目.work测试文件，不启动任何真实Agent。

Core的evaluate始终不给许可，report_event只回复validated_not_persisted或duplicate。不存在把用户确认、Hook返回或合法JSON当作执行成功的路径。没有实现I05/I06 ledger，不能声称nonce原子消费、消费后崩溃恢复或业务副作用exactly-once。

## 与产品初心的对应

用户能通过中文错误理解身份不一致、内容不符、等待超时、授权尚不可用和合成范围限制。七款状态仍L0/五false。UI、真实控制和OS隔离只有在后续取得对应独立证据后才能展示。当前工作集中于防止未来误授权与误报的基础，不以监控事件数量或接口存在替代实际控制。

## 剩余重大决策

D01–D05保留。本批I03已PASS，下一批I04/I05的直接前置已满足。本批结束，完整授权链、I02-B隔离环境及I07资源实验按后续任务安排。

[测试与交付](FINAL-REPORT.md) [新问题](NEW-FINDINGS.md) [进度](PROGRESS.md)

