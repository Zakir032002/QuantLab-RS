# Requirement traceability

Every requirement maps to an executable test. CI checks the mapping.

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
| REQ-CHECK-011 | `req_check_011_every_fill_needs_later_cash_and_same_asset_position` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-CHECK-012 | `req_check_012_i2_uses_local_claim_windows_for_carried_errors` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-CHECK-013 | `req_check_013_sparse_indices_must_be_unique_and_in_sequence_order` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-CHECKPOINT-001 | `req_checkpoint_001_earlier_bad_claim_is_an_opening_boundary` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-CHECKPOINT-002 | `req_checkpoint_002_open_and_terminal_orders_and_prior_trades_survive_cut` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-CHECKPOINT-003 | `req_checkpoint_003_invalid_context_never_becomes_a_financial_accusation` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-CLI-001 | `req_cli_001_exit_codes_and_bundle_commands` | [exit_codes.rs](../crates/qlrs-cli/tests/exit_codes.rs) |
| REQ-CLI-002 | `req_cli_002_limits_preserve_verified_output_without_minimality_claim` | [exit_codes.rs](../crates/qlrs-cli/tests/exit_codes.rs) |
| REQ-FP-001 | `req_fp_001_canonical_vector_and_order_independence` | [fingerprint.rs](../crates/qlrs-check/src/fingerprint.rs) |
| REQ-FP-002 | `req_fp_002_at_seq_filters_status_and_anchor` | [lib.rs](../crates/qlrs-check/src/lib.rs) |
| REQ-FP-003 | `req_fp_003_all_fields_have_distinct_canonical_encodings` | [fingerprint.rs](../crates/qlrs-check/src/fingerprint.rs) |
| REQ-FP-004 | `req_fp_004_utf8_lengths_sign_and_zero_multiset_entries` | [fingerprint.rs](../crates/qlrs-check/src/fingerprint.rs) |
| REQ-GATE-001 | `req_gate_001_all_eleven_defects_on_rich_profiles` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-GENERATOR-001 | `req_generator_001_interleaving_sparse_claims_long_lived_and_carried_offsets` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-GENERATOR-002 | `req_generator_002_rich_statuses_across_seeds` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-GENERATOR-003 | `req_generator_003_each_instrument_has_independent_buy_sell_cycles` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-GRAPH-001 | `req_graph_001_sell_closures_consume_earlier_buy_inventory` | [reduction.rs](../crates/qlrs-testkit/tests/reduction.rs) |
| REQ-GRAPH-002 | `req_graph_002_checkpoint_inventory_replaces_historical_buy_dependencies` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
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
| REQ-REDUCE-005 | `req_reduce_005_limits_return_best_verified_uncertified_bundles` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-REDUCE-006 | `req_reduce_006_sparse_dense_and_incremental_patterns_match_on_all_defects` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-REDUCE-007 | `req_reduce_007_other_lifecycle_breaches_are_not_admitted_or_returned_on_limits` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |
| REQ-REDUCE-008 | `req_reduce_008_short_logs_read_legacy_removed_unit_lists` | [corrections.rs](../crates/qlrs-testkit/tests/corrections.rs) |

| REQ-CHECK-014 | `req_check_014_violation_dominates_uncovered_tail_and_is_reducible` | [part2.rs](../crates/qlrs-testkit/tests/part2.rs) |
| REQ-REDUCE-009 | `req_reduce_009_i2_witness_needs_only_its_instrument_and_position_claims` | [part2.rs](../crates/qlrs-testkit/tests/part2.rs) |
| REQ-REDUCE-010 | `req_reduce_010_r1_d04_sizes_finish_under_existing_budgets` | [part2.rs](../crates/qlrs-testkit/tests/part2.rs) |

| REQ-REDUCE-011 | `req_reduce_011_cache_forced_collision_and_incremental_toggles` | [test](../crates/qlrs-reduce/src/cache.rs) |

| REQ-REDUCE-012 | `req_reduce_012_long_lived_and_second_carried_error_are_bounded` | [test](../crates/qlrs-testkit/tests/part2.rs) |

| REQ-REDUCE-013 | `req_reduce_013_algebra_matches_full_checker_on_closed_subsets_per_shape` | [test](../crates/qlrs-testkit/tests/differential_part2.rs) |

| REQ-REDUCE-014 | `req_reduce_014_evaluate_batch_selects_success_and_obeys_limits` | [test](../crates/qlrs-reduce/src/lib.rs) |

| REQ-REDUCE-015 | `req_reduce_015_remove_closure_respects_dependents_pins_and_noops` | [test](../crates/qlrs-reduce/src/lib.rs) |

| REQ-GATE-002 | `req_gate_002_shapes_have_real_removals_and_recheckable_certificates` | [test](../crates/qlrs-testkit/tests/part2.rs) |

| REQ-BUNDLE-002 | `req_bundle_002_v1_roles_and_certificates_reject_false_proofs` | [test](../crates/qlrs-testkit/tests/part2.rs) |

| REQ-BUNDLE-003 | `req_bundle_003_original_v0_writer_bundle_still_verifies` | [test](../crates/qlrs-testkit/tests/part2.rs) |

| REQ-REDUCE-016 | `req_reduce_016_unit_renaming_preserves_observed_witness` | [test](../crates/qlrs-testkit/tests/part2.rs) |

| REQ-REDUCE-017 | `req_reduce_017_nonmonotone_superset_is_saved_and_seed_recovers` | [test](../crates/qlrs-testkit/tests/part2.rs) |

| REQ-REDUCE-018 | `req_reduce_018_graph_commitment_is_canonical_and_rle_is_exact` | [test](../crates/qlrs-reduce/src/lib.rs) |
