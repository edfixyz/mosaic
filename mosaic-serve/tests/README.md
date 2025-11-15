# Concurrency Integration Tests

This directory contains integration tests that verify the concurrency fix for mosaic-serve.

## Overview

The tests verify that multiple backend requests can execute concurrently instead of blocking each other. Before the fix, requests were serialized because a coarse-grained mutex was held across async await points.

## Running the Tests

**Note**: Most integration tests require a running Miden node infrastructure and are marked with `#[ignore]` by default. They won't run in standard CI but can be run locally when you have the proper setup.

Run tests that don't require infrastructure (runs in CI):
```bash
cargo test -p mosaic-serve --test concurrency_tests
```

Run ALL tests including ignored ones (requires Miden node):
```bash
cargo test -p mosaic-serve --test concurrency_tests -- --ignored --nocapture
```

Run a specific ignored test with output:
```bash
cargo test -p mosaic-serve --test concurrency_tests test_concurrent_account_creation_different_users -- --ignored --nocapture
```

## Test Cases

### Lightweight Tests (Run in CI)

#### `test_concurrent_desk_operations`
- **Purpose**: Verify concurrent desk cache access doesn't cause deadlocks
- **Expected**: All 10 concurrent desk list operations complete successfully
- **Key metric**: No panics or deadlocks
- **Infrastructure**: None required (just tests locking behavior)

### Heavy Tests (Require Miden Infrastructure - Marked `#[ignore]`)

#### 1. `test_concurrent_account_creation_different_users`
- **Purpose**: Verify two different users can create accounts simultaneously
- **Expected**: Both operations succeed and create distinct accounts
- **Key metric**: Both complete without serialization
- **Requires**: Miden node running

#### 2. `test_concurrent_operations_do_not_block`
- **Purpose**: Verify slow operations (create account) don't block fast operations (list accounts)
- **Expected**: List operation completes much faster than create operation
- **Key metric**: `list_time < create_time`
- **Requires**: Miden node running

#### 3. `test_concurrent_get_client_same_user_network`
- **Purpose**: Test the double-check pattern prevents duplicate client spawns
- **Expected**: 10 concurrent `get_client()` calls for same (secret, network) only spawn ONE client
- **Key metric**: All handles work correctly, no duplicate spawns
- **Requires**: Miden node running

#### 4. `test_high_concurrency_stress`
- **Purpose**: Stress test with 20 concurrent account creations
- **Expected**: At least 15/20 succeed (some may fail due to test env resource constraints)
- **Key metric**: No deadlocks, data integrity maintained
- **Requires**: Miden node running

#### 5. `test_timing_proves_concurrency`
- **Purpose**: Prove through timing that requests execute concurrently
- **Expected**: Concurrent execution is at least 1.3x faster than sequential
- **Key metric**: `speedup > 1.3x`
- **Requires**: Miden node running

## What the Tests Verify

✅ **True concurrency**: Requests don't block each other
✅ **Interior mutability**: Brief lock acquisition for cache access only
✅ **Race condition safety**: Double-check pattern works correctly
✅ **No deadlocks**: Concurrent access to shared state is safe
✅ **Data integrity**: Concurrent operations don't corrupt data
✅ **Performance**: Concurrent is significantly faster than sequential

## Test Environment

- Uses `tempfile::TempDir` for isolated test storage
- Each test creates its own `Serve` instance
- Tests use `tokio::spawn` for true concurrent execution
- Uses `futures::future::join_all` to wait for concurrent tasks

## Interpreting Results

### Success Indicators
- All tests pass
- Timing test shows speedup > 1.3x
- No panics or deadlocks in stress test

### Failure Indicators
- Tests timeout (indicates deadlock)
- Timing test shows speedup < 1.3x (indicates serialization)
- Data corruption (different accounts get same ID)

## Notes

- Some operations in stress test may fail due to resource constraints (expected)
- Timing tests may vary based on system load
- Tests create temporary directories that are cleaned up automatically
