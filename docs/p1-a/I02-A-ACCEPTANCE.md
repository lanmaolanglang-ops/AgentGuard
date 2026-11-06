# I02-A节点验收

AgentGuard V2.0 · P1-A · 2026-11-06

## 结论与前置

PASS。I01已通过，I02-A仅处理纯数据。设计基线仍42份原字节，未进行I02-B的VM/网络/真实Agent实验。

## 实际修改文件

新增tools/extract_protocol.py、tools/check_fixtures.py、tools/__init__.py、tests/python/test_fixtures.py、protocol/schemas下9个JSON、fixtures/p0.1.json与virtual-tree.json，以及evidence/I02-A下schema-results.json/frozen-reference-results.json、本报告与进度索引。Schema/夹具由冻结Markdown原样提取，--check逐对象/文本验证来源。

## 架构与依赖

只建立合成输入与验证工具，没有Core授权或Worker实现。使用I20-A固定的标准jsonschema 4.26.0及format组件，禁止外部Schema检索。虚拟文件树仅JSON描述，未物化D盘GuardLab路径，sink.invalid未请求。

## 测试

提取9 Schema/65 fixture，标准validator检查65项全部符合预期，37正例通过、28反例拒绝，FAIL 0。冻结参考runner另通过8迁移、64状态对、13 parser拒绝、1 parser正例、6语义断言组及3版本拒绝。5项Python回归覆盖来源一致、65夹具、date-time/uuid强制启用、禁止外部Schema与虚拟树边界。连同I01当前13项Python测试全部PASS。

## 安全影响和缺口

新增格式依赖回归修复了原验证环境date-time可静默跳过的问题，冻结规范字段没有改变。该缺口属于验证实现/依赖修正。对应Issue I02-A，AT21/29/34/45纯数据子范围。nonce并发、真实身份/句柄与实际执行副作用仍待后续节点。

[结构验证结果](evidence/I02-A/schema-results.json) [冻结参考结果](evidence/I02-A/frozen-reference-results.json)
