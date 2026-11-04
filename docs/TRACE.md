# Requirement traceability

Every requirement maps to an executable test. The routine CI checks this mapping.

| Requirement | Test | Source |
|---|---|---|
| REQ-BUNDLE-001 | `req_bundle_001_deterministic_roundtrip_and_every_file_tamper` | [reduction.rs](../crates/qlrs-testkit/tests/reduction.rs) |
| REQ-CHECK-001 | `req_check_001_catalogue_statuses_and_invariants` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-002 | `req_check_002_clean_generated_consistent` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-003 | `req_check_003_all_defects_1000_injection_points` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-004 | `req_check_004_residual_relations_are_hints` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-005 | `req_check_005_clean_corpus_and_flat_position` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-006 | `req_check_006_unknown_event_is_unsupported_not_input_error` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-007 | `req_check_007_partial_and_status_precedence` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-008 | `req_check_008_lifecycle_subcodes_and_supported_terminal_states` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-009 | `req_check_009_unknown_fill_never_invents_a_cash_delta` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CHECK-010 | `req_check_010_trade_uniqueness_needs_known_instrument_scope` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-CLI-001 | `req_cli_001_exit_codes_and_bundle_commands` | [exit_codes.rs](../crates/qlrs-cli/tests/exit_codes.rs) |
| REQ-FP-001 | `req_fp_001_canonical_vector_and_order_independence` | [lib.rs](../crates/qlrs-check/src/lib.rs) |
| REQ-FP-002 | `req_fp_002_at_seq_filters_status_and_anchor` | [lib.rs](../crates/qlrs-check/src/lib.rs) |
| REQ-GRAPH-001 | `req_graph_001_sell_closures_consume_earlier_buy_inventory` | [reduction.rs](../crates/qlrs-testkit/tests/reduction.rs) |
| REQ-IO-001 | `req_io_001_strict_unknown_fields_decimal_types_and_sequences` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-IO-002 | `req_io_002_roundtrip_verbatim` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-IO-003 | `req_io_003_verbatim_noncanonical_decimal_evidence` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-IO-004 | `req_io_004_malformed_overflow_still_input_error` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-IO-005 | `req_io_005_cash_only_header_needs_no_instruments` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-IO-006 | `req_io_006_escaped_strings_use_owned_cold_path_and_roundtrip` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-IO-007 | `req_io_007_overflow_is_not_a_zero_quantity_or_precision_accusation` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-LEDGER-001 | `req_ledger_001_zero_allocations_in_replay_loop` | [allocations.rs](../crates/qlrs-testkit/tests/allocations.rs) |
| REQ-LEDGER-002 | `req_ledger_002_replay_is_independent_of_observed_claims` | [invariants.rs](../crates/qlrs-testkit/tests/invariants.rs) |
| REQ-NUM-001 | `req_num_001_decimal_grammar_and_per_value_scale` | [lib.rs](../crates/qlrs-num/src/lib.rs) |
| REQ-NUM-002 | `req_num_002_checked_operations_and_parser_never_panic` | [lib.rs](../crates/qlrs-num/src/lib.rs) |
| REQ-NUM-002 | `req_num_002_signed_limits_and_checked_overflow` | [lib.rs](../crates/qlrs-num/src/lib.rs) |
| REQ-NUM-003 | `req_num_003_all_rounding_modes_signed_ties` | [lib.rs](../crates/qlrs-num/src/lib.rs) |
| REQ-NUM-004 | `req_num_004_wide_intermediate_can_fit_final_value` | [lib.rs](../crates/qlrs-num/src/lib.rs) |
| REQ-NUM-005 | `req_num_005_compare_by_upscaling_without_overflow` | [lib.rs](../crates/qlrs-num/src/lib.rs) |
| REQ-NUM-006 | `req_num_006_roundtrip_100k_values` | [lib.rs](../crates/qlrs-num/src/lib.rs) |
| REQ-NUM-007 | `req_num_007_mul_rescale_matches_bigint` | [properties.rs](../crates/qlrs-num/tests/properties.rs) |
| REQ-NUM-008 | `req_num_008_parse_format_roundtrip` | [properties.rs](../crates/qlrs-num/tests/properties.rs) |
| REQ-NUM-009 | `req_num_009_parser_no_panics` | [properties.rs](../crates/qlrs-num/tests/properties.rs) |
| REQ-NUM-010 | `req_num_010_checked_mul_exact_product_scale` | [properties.rs](../crates/qlrs-num/tests/properties.rs) |
| REQ-NUM-011 | `req_num_011_notional_fee_round_once` | [properties.rs](../crates/qlrs-num/tests/properties.rs) |
| REQ-NUM-012 | `req_num_012_product_plus_fee_bigint_oracle` | [properties.rs](../crates/qlrs-num/tests/properties.rs) |
| REQ-NUM-013 | `req_num_013_error_reason_codes` | [properties.rs](../crates/qlrs-num/tests/properties.rs) |
| REQ-REDUCE-001 | `req_reduce_001_three_defects_exact_replay_and_idempotence` | [reduction.rs](../crates/qlrs-testkit/tests/reduction.rs) |
| REQ-REDUCE-002 | `req_reduce_002_actual_closure_minimality_certificate` | [reduction.rs](../crates/qlrs-testkit/tests/reduction.rs) |
| REQ-REDUCE-003 | `req_reduce_003_property_preserves_instance_and_source` | [reduction.rs](../crates/qlrs-testkit/tests/reduction.rs) |
| REQ-REDUCE-004 | `req_reduce_004_sparse_fast_predicate_matches_full_replay` | [reduction.rs](../crates/qlrs-testkit/tests/reduction.rs) |
