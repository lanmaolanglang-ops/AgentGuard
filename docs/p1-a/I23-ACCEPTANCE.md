# I23节点验收

AgentGuard V2.0 · P1-A · 2026-11-06

## 结论与前置

PASS，I01已通过。验收采用冻结合同允许的“固定依据或明确关闭相关能力”路径。所有七款继续L0、五false，未关闭需要真实实验的Q01–Q24或D01–D05。

## 文件与操作

新增tools/review_sources.py、tools/review_conflicts.py、evidence/I23/source-reachability.json、fixed-source-excerpts.json、本报告与进度索引。只读公开GET与固定GitHub源文件读取，不执行仓库或Agent代码。公开源码原始域直连失败后，使用GitHub contents API读取同一固定commit。

## 来源与冲突

共复核S01–S58/N01–N18的76条来源，68条HTTP200，8条不可达/403。其中S01–06/S57–58保留历史证据，Codex接口相关能力全部禁用，等待具体构建联调。HTTP200不作为运行证据。

固定Reasonix提交f4f32cc7d3078f7031c7852fbf0b7de410958199的GUIDE明确native hook不能回答审批，PreToolUse timeout与spawn失败语义不同，Extension Protocol v2的full-trust allow可覆盖host deny。GUIDE含windows_integrity opt-in说明，同时保留Windows无bash sandbox旧段落，CONFIG_PATHS也称OS沙箱外。保留矛盾并禁用Reasonix强参数绑定/自动许可/OS认证，未对本机2.56.0作推断。

WorkBuddy桌面payload/verdict未确立，不引用CodeBuddy CLI合同。ZCode用户级/project Hook与timeout未映射具体安装构建，PE Electron版本不作产品版本，相关能力关闭。豆包仅按用户确认App内工作模式登记，Windows构建/接口保持未知。DSH固定源码文档不替代release bundle，guard不作OS边界。

## 质量与验收证据

来源自动回归2项通过、0项失败，分别检查76项来源的完整状态/无运行声明，以及3份固定commit/hash片段。访问失败8项仍作为来源缺口记录。

方向检查满足核心目标，提供用户可理解的未知与限制，零接入攻击面新增，P0.1边界无变化。76条来源均有状态记录，3份固定源码excerpt+hash可追溯，7行清单及五false在I01测试中通过。来源访问失败8项按未知记录，未伪造PASS。对应Issue I23，AT20/24/28/35只提供前置证据，不记为运行通过。

[来源可达性](evidence/I23/source-reachability.json) [固定来源片段](evidence/I23/fixed-source-excerpts.json) [当前身份](evidence/I01/agent-inventory.json)
