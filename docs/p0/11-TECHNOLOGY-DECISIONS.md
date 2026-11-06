# 技术选型与 ADR

AgentGuard V2.0 · P0-5 · 2026-11-05

## 目的

逐项审查候选栈并记录取舍、成本、验证条件和重审触发点。所有“采纳”为设计决定，P0 没有安装依赖。

## 结论

采纳 Tauri 2、React、TypeScript、Rust、SQLite、Named Pipes 和 Windows 原生 API。UI 与安全 Core 分进程，策略与授权账本同进程同事务域。Adapter 按产品隔离，Hook shim 尽量采用独立签名二进制。高级 OS 隔离路线作为发布前门槛。

| ADR | 决策 | 主要替代方案 | 成本/风险/收益与重审条件 |
|---|---|---|---|
| ADR-001 | Tauri 2 桌面壳 | Electron、原生 WinUI | 本产品无大型 Node 主进程需求，已有 Rust Core，Tauri capability 方便缩小 UI 权限。成本为 Rust/Windows 集成与 WebView2 版本差异。Electron 的统一 Chromium 与生态利于开发，但打包/更新与 Node 桥维护面较大。若 WebView2 无法可靠支持无障碍/多窗才重审 |
| ADR-002 | React + TypeScript | 原生 UI、其它前端框架 | 多窗审批、复杂状态适合显式状态建模，团队栈成本较低是设计假设。前端无权限决策，类型不能替代 IPC runtime 校验。性能不足先优化数据投影再重选 |
| ADR-003 | Rust Core | Go、C#/.NET、C++ | 内存安全与 Win32 FFI 适于本地 daemon。Go/C# 也可满足业务层，但本次优先单语言复用底层结构。unsafe、解析器、逻辑错误仍需审查。若 Windows API 支持成本失控可重审 C# broker |
| ADR-004 | 独立用户态 Core 后台进程 | Tauri UI 内部逻辑 | UI 崩溃不关闭策略，需额外启动/重启、IPC 及退出语义设计。不能仅靠名为 sidecar 宣称独立。UI-only demo 无法满足本需求 |
| ADR-005 | 第一版无 Windows Service | LocalSystem/专用账户 service | 遵守无默认管理员/服务要求，减少高权限入口。成本为用户登出后不运行、抗同权限进程弱。企业常驻/隔离账户需求在另行授权的高级版本重审 |
| ADR-006 | Named Pipes 本地传输 | loopback TCP、stdio-only | Windows DACL/logon session/PID 认证便利，无需监听浏览器可访问端口。需明确 DACL、远程拒绝和抢占防护。跨平台时重新比较 TCP/Unix socket |
| ADR-007 | SQLite WAL + 单 writer | 多进程直接写、PostgreSQL、外部 broker | 本地无运维，事务把授权计数和审计绑在一起。WAL 仍单写者，必须处理 busy/checkpoint。吞吐目标实测不达再考虑存储分层 |
| ADR-008 | 独立 Adapter 与小 Hook shim | 全部加载到 Core、脚本 Hook | 缩小解析器故障影响，崩溃独立。宿主 runtime 插件可能仍须在宿主内存在桥接，不能视作 OS 隔离。首次可审查固定版本二进制，避免依赖本机 Python/Node 搜索路径 |
| ADR-009 | 本地有界队列 + SQLite durable ledger | Kafka/RabbitMQ/Redis | 减少安装和运维。控制与观察分队列，优先确保 grant 记录，观察丢失须可见。不能用无界内存保证“永不丢事件” |
| ADR-010 | 策略模块与 ledger 共置独立 Core | 再拆单独 policy daemon | UI 已隔离。再拆会引入分布式 grant 消费、崩溃和撤销竞态，首版无必要收益。未来允许不可信策略语言时单独 sandbox worker |
| ADR-011 | 本地模板解释优先 | 远端 LLM 唯一判断、默认本地大模型 | 无额外 API、延迟和隐私负担低。复杂命令解释不完整时显示未知，保持确定性拒绝。可选模型解释独立 worker，不可产生许可 |
| ADR-012 | 原生沙箱复用 + AppContainer/受限进程实验 | 仅 Hook、默认驱动、整机注入 | 兼顾可行性和验证。前置 Hook 覆盖范围分级，强隔离需要兼容性成本。没有可靠边界时保持低保护级别 |
| ADR-013 | 默认不写原生长期 allow 规则 | 将一次批准同步为用户级白名单 | 保留撤销/计数控制，减少越权。若产品只能持久许可则不提供自动长期授权 |
| ADR-014 | 精确版本/工具 provider profile | 仅依据产品名字选择通用 Adapter | 避免滚动文档漂移，需维护兼容性测试矩阵。新构建默认待验，允许观察导入但不继承绿色状态 |
| ADR-015 | 脱敏 append-only 审计 + 可导出锚点 | 全量原文日志、宣传不可篡改 | 减少秘密泄露，支持篡改线索。独立锚点保存有用户成本，不能证明边界外事件完整性 |
| ADR-016 | 明确的能力声明 + 失败状态 | 七款共用单一“已保护”开关 | 界面复杂度增加，但能准确表达语义、拦截、OS 隔离和证据缺失 |

## 技术依据

Tauri 多进程模型与 capability、Electron 安全规范、SQLite WAL 限制、Named Pipe DACL 以及 Microsoft 安全 API 构成选型依据。[S40–56](13-SOURCE-REGISTER.md)

各 Agent 的前置失败语义及 full-trust 插件边界决定 ADR-004、008、012–014。具体事实与部署状态见独立调查。[接口调查](04-AGENT-CAPABILITY-MATRIX.md)

## 依赖与供应链原则

P1 建立 Cargo/npm lockfile、SBOM、签名发布、依赖安全审查和最低允许版本。版本号在 P0 不凭空固定。React/UI 不加载远端脚本或不可信 HTML，Rust FFI 单点封装并审查。SQLite 构建需核对上游已知 WAL 缺陷修复与实际 bundled 版本，不能只写“使用最新”。

## 风险

选择 Tauri 不能自动缩小外部 Agent 权限。Rust 不保证安全策略正确。无 Windows Service 的产品无法声称抗所有本地恶意软件。多进程的复杂度必须用端到端契约测试控制。

## 尚未确定的内容

WebView2 最低版本、Rust/SQLite 具体稳定版、Windows API binding、安装签名方案、性能预算和兼容版本维护成本，在 P1-I01/I03/I05/I20 中验证。[P1 计划](14-P1-DEVELOPMENT-PLAN.md)

