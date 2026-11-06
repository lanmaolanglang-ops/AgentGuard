# I06-D 验收矩阵

执行状态以同一提交的本地报告和 Windows CI `summary.json` 为准。单项未执行、失败或无法核验均阻断交付状态。

| 门禁 | 证据 |
|---|---|
| D1 Parent Rule | `role_checks_and_rule_is_not_automatic_approval`、`rule_budget_is_rechecked_at_reserve_with_two_pending_grants`、`rule_version_and_exact_scope_and_revoke` |
| D2 Grant | `frozen_grant_schema_agrees_on_all_64_transition_pairs`、`manual_issue_consume_once_and_no_lookup_ticket`、`revoke_or_reject_does_not_refund_or_reissue` |
| D3 Reservation | `nonce_collision_is_atomic_and_never_reclaims_terminal_nonce`、`unknown_commits_are_lookup_only_and_keep_nonce_and_quota` |
| D4 Permit | `wire_schema_and_parser_reject_before_consuming`、`correct_permit_and_operation_macs_cannot_override_stored_binding`、`prepared_route_replay_is_denied_and_trusted_retirement_cannot_be_reverted` |
| D5 Prepared | `prepared_snapshot_mutations_permanently_retire_grant`、`prepared_generation_deadline_and_decision_changes_invalidate_at_reserve` |
| D6 决策和时间 | `deny_and_unknown_fail_closed_at_preparation_and_redemption`、`utc_monotonic_rollback_and_clock_unavailability_never_extend_tickets`、`ttl_limits_prepared_and_approval_deadlines`、`separate_epochs_and_os_reboot_invalidate_all_pending_authority`、`utc_fixture_formatter_handles_leap_century_and_limits` |
| 六域和轮换 | `six_composed_macs_match_independent_references`、`semantic_secret_shadow_preserves_unicode_null_and_other_fields`、`secret_validation_rejects_unregistered_and_ambiguous_pointers`、`intent_payload_has_frozen_fields_and_one_claim`、`audit_mac_is_typed_and_bound_to_event_segment_previous_key_and_generation`、`rotation_requires_new_id_generation_epoch_and_material` |
| 原合同与能力基线 | 42 原设计文件 bytes 和 SHA-256、原 Schema 和 fixture 路径差异检查、七款 L0 与五项 false、I03 的 21 组固定向量。 |
| Windows 回归 | `verify_i06_d.py` 重跑 G0、I05、C2 和第一批。锁文件校验 D 注册表依赖为原锁定集合的子集。 |
| 执行边界 | 独立 Rust workspace。Core／Ledger Cargo 配置、RPC、生产入口无改动。实际运行证据仅来自合成测试和既有回归。 |

完整结果及证据获取方式见 [测试报告](I06-D-TEST-REPORT.md)。本矩阵用于工程交付，负责人独立结论单列。
