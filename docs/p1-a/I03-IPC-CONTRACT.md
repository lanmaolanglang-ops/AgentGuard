# I03合成协议与IPC实现合同

AgentGuard V2.0 · P1-A · 2026-11-06

## 实现范围

本文件细化05/06/07已经要求的有界Named Pipes、身份、角色与协议校验。正式event-v2、operation、permit、grant与typed encoding保持P0.1原字节。当前Core仅提供validate和serve-synthetic入口，真实执行与授权发行按I04/I05/I06后续交付。

## 帧与连接

每帧为4字节u32 big-endian payload字节数，后跟UTF-8 JSON，长度1至1048576。读取在分配前检查长度，每次读/写与等待连接有5秒deadline。请求最多16帧，重复键、浮点/指数/-0、不安全整数、非法Unicode、深度超过8、数组超过128和map超过64在Schema前拒绝。正式Schema还有更小字段约束。

只允许\\\\.\\pipe\\AgentGuard.Synthetic.<UUID>。服务端first-instance、单实例、拒绝remote clients，显式DACL只给当前logon SID。客户端指定预期Core PID和SHA256，服务端指定合成probe SHA256，双方从Windows获取peer PID并查询创建时间、用户SID、logon SID/LUID、IL、restricted状态与token modified LUID。保留真实进程句柄，每次请求重新核对实例/token/image。客户端自报整数不能作为HANDLE。

身份读取及SDDL/pipe创建的unsafe集中在windows.rs，使用OwnedHandle与LocalFree RAII，TokenInformation缓冲区保留至SID读取结束。身份数据只用于这条合成连接，不填充任何真实Agent认证。

## 内部请求

Core握手返回synthetic_hello、major=2、minor=0和BCryptGenRandom生成的32字节challenge十六进制。请求闭合字段为major/minor/sequence/challenge/method/payload。sequence从1递增，绑定当前连接challenge。角色由已核验进程与Core配置选择，客户端没有role字段。

| 方法 | 当前权限与行为 |
|---|---|
| heartbeat | 所有内部角色可读合成状态，payload必须为空对象，始终L0/authorization_available=false |
| report_event | 仅Adapter/Sensor角色提交合成Observation，闭合字段为source_event_id/agent_id/tool_name/command/arguments。通过只回复validated_not_persisted或duplicate，不表示durable审计或执行成功 |
| evaluate | 仅Adapter可到达入口，当前统一返回authorization_unavailable，不发行permit |
| 其它方法 | 请求解析拒绝，Prepare/Redeem/Launch等执行合同等待I06 |

Observation的source_event_id按128个Unicode字符计，tool_name与command使用07相同nullable及长度约束，arguments以types-v2的value定义校验。policy/risk/cert/grant/OS身份/digest/HANDLE等客户端字段均因闭合结构被拒绝。

source_event_id在单连接内去重，同ID内容改变拒绝。持久source ledger、重启恢复与跨连接授权消费由I05/I06/I22交付，当前进程没有授权状态可复活。断开/超时后连接失效，重连必须新challenge。错误返回固定中文code/message，不回显命令、参数或假秘密值。

## 校验与迁移

Rust标准jsonschema固定Draft2020-12，启用format，资源retriever只读取嵌入9份Schema的本地URN map，关闭网络/文件默认features。Schema是可信构建输入，操作帧先严格解析。CLI的schema_valid只表达结构结果，authorization_available=false。

Rust迁移API验证legacy schema，保留原事件ID与raw字节SHA256关联，声明身份进入reported，旧许可和OS认证清除，事件成为imported。无法在新字段边界内完整表示时拒绝导入并要求保留原记录，避免静默截断。所有8冻结legacy样本通过。

密码学库仅提供六个固定域的typed encoding/frame/HMAC及constant-time verify_slice。当前使用公开假key测试，尚无生产key存储或授权IPC签名接口。Rust/Python/Node共同核对21向量。

## 验收与剩余范围

14个Rust合同测试、65固定fixture、21跨语言向量和64状态对，以及13个真实合成Core/probe进程与CLI入口测试。测试覆盖身份/image/PID不符、管道抢占、超时/中断、重放、伪allow/身份、超长/截断、source冲突与CLI有界读取。它们提供I03对应AT21/23/29/34的子范围证据。

跨Windows用户、不同IL、远程客户端负测、Agent Host关联、真实token隔离、独立Observer、persistent grant并发/崩溃恢复和实际副作用边界均未认证。D01–D05维持待决。[冻结执行合同](../p0.1/15-EXECUTION-BROKER-CONTRACT.md) [能力定义](../p0.1/16-PROTECTION-CAPABILITY-CONTRACT.md)
