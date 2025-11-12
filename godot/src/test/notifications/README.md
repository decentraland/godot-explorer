# Notification Queue Tests

Comprehensive tests for the local notification queue management system.

## Overview

These tests verify the correct behavior of the notification queue system, including:

1. **Adding notifications** to the queue
2. **Deleting notifications** from the queue
3. **Queue management** when full (24 notifications maximum)
4. **Priority scheduling** based on trigger timestamps
5. **Queue synchronization** on app launch/refocus
6. **Image handling** with separate blob storage
7. **Expired notification cleanup**

## Test Structure

### Mock Plugin (`mock_notifications_plugin.gd`)

The mock plugin simulates both iOS (`DclGodotiOS`) and Android (`GodotAndroidPlugin`) interfaces:

- **In-memory storage**: All notifications stored in dictionaries
- **Database API**: Full CRUD operations with WHERE clause support
- **OS API**: Simulated scheduling/canceling of notifications
- **Test utilities**: Inspection and reset capabilities

### Test Suite (`test_notifications_queue.gd`)

Comprehensive test scenarios covering all queue operations:

#### Test Case 1: Add Single Notification
- Verifies basic insertion and scheduling

#### Test Case 2: Delete Notification
- Tests removal from both database and OS

#### Test Case 3: Add When Queue Full
- Adds 25th notification that comes AFTER the 24th
- Validates it's NOT scheduled with OS

#### Test Case 4: Delete When Queue Full
- Removes a scheduled notification
- Validates pending notification gets scheduled

#### Test Case 5: Add Notification After 24th Position
- Adds notification with later timestamp than 24th
- Verifies it remains unscheduled

#### Test Case 6: Add Notification Before 24th Position ⭐
- **Most important test**: Adds notification that should be at position 15
- Verifies it gets scheduled and bumps out the 25th notification
- Validates the 24th notification is NOT evicted

#### Test Case 7: Queue Sync on App Launch
- Simulates app restart with mix of expired and future notifications
- Validates expired cleanup

#### Test Case 8: Cancel Queued Notification
- Tests cancellation API

#### Test Case 9: Expired Notification Cleanup
- Validates selective removal of expired notifications

#### Test Case 10: Queue with Images
- Tests separate image blob storage
- Validates images are not loaded in regular queries

#### Test Case 11: End-to-End Sync Algorithm Validation ⭐⭐⭐
- **Most comprehensive test**: Validates the complete `_sync_notification_queue()` algorithm
- Creates complex scenario: 5 expired, 30 future notifications, inconsistent states
- Tests orphan cleanup (OS notifications not in DB)
- Validates all 6 steps of the sync algorithm
- Ensures DB and OS states are perfectly consistent after sync

#### Test Case 12: Sync with Identical Timestamps
- Tests behavior when multiple notifications have the same trigger time
- Validates deterministic scheduling order
- Ensures all are scheduled when total < 24

#### Test Case 13: Sync Consistency Validation
- Uses helper function `_validate_db_os_consistency()`
- Validates that DB scheduled count matches OS scheduled count
- Ensures all OS IDs exist in DB as scheduled
- Ensures all DB scheduled IDs exist in OS
- Validates count is within MAX_OS_SCHEDULED limit

#### Test Case 14: Sync with Mixed States
- Complex scenario: expired + scheduled + pending notifications
- Tests that sync correctly handles state transitions
- Validates pending notifications get promoted when slots available
- Ensures expired are removed and scheduled are maintained

## Running the Tests

### Automated Tests (CI)

The notification system is tested in CI via Rust integration tests located in `lib/src/notifications/mod.rs`. These tests run automatically as part of `cargo run -- coverage` and validate:

1. NotificationsManager singleton exists and is accessible
2. Mock plugin can be instantiated and used
3. Basic database operations work correctly

These tests run in CI on every PR.

### Comprehensive Manual Tests

The full notification queue test suite with all 10 test scenarios can be run manually:

#### Option 1: Run from Godot Editor

1. Open the project in Godot Editor
2. Run the test scene:
   ```
   Scene -> Run Scene (F6)
   ```
   With the file `godot/src/test/notifications/test_notifications_queue.tscn` open

3. Check the Output panel for results

#### Option 2: Run from Command Line

```bash
# From project root
cargo run -- run -- --path godot/src/test/notifications/test_notifications_queue.tscn
```

These comprehensive tests should be run manually before merging changes to the notification system.

## Expected Output

```
========================================
Starting Notification Queue Tests
========================================

Running: Add single notification to queue
  Result: PASS

Running: Delete notification from queue
  Result: PASS

Running: Add notification when queue is full
  Result: PASS

Running: Delete notification when queue is full
  Result: PASS

Running: Add notification after 24th position
  Result: PASS

Running: Add notification before 24th position
  Result: PASS

Running: Queue sync on app launch
  Result: PASS

Running: Cancel queued notification
  Result: PASS

Running: Expired notification cleanup
  Result: PASS

Running: Queue with images
  Result: PASS

========================================
All tests completed
========================================

========================================
TEST RESULTS SUMMARY
========================================

✓ PASS: Add single notification to queue
✓ PASS: Delete notification from queue
✓ PASS: Add notification when queue is full
✓ PASS: Delete notification when queue full
✓ PASS: Add notification after 24th position
✓ PASS: Add notification before 24th position
✓ PASS: Queue sync on app launch
✓ PASS: Cancel queued notification
✓ PASS: Expired notification cleanup
✓ PASS: Queue with images

========================================
Total: 10/10 tests passed (100.0%)
========================================

All tests passed!
```

## Modifying Tests

To add new test scenarios:

1. Create a new test function in `test_notifications_queue.gd`:
   ```gdscript
   func test_my_new_scenario() -> void:
       var test_name = "My new scenario"
       print("Running: %s" % test_name)

       mock_plugin.reset()

       # Test logic here...

       var passed = true  # Your assertion

       record_test_result(test_name, passed, "Description")
       print("  Result: %s\n" % ("PASS" if passed else "FAIL"))
   ```

2. Call it from `run_all_tests()`:
   ```gdscript
   await test_my_new_scenario()
   ```

## Mock Plugin API

The mock plugin provides the same API as real plugins:

### Database Operations
```gdscript
# Insert notification
mock_plugin.db_insert_notification(id, title, body, trigger_timestamp, is_scheduled, data, image_base64)

# Query notifications
var results = mock_plugin.db_query_notifications(where_clause, order_by, limit)

# Delete notification
mock_plugin.db_delete_notification(id)

# Clear expired
var count = mock_plugin.db_clear_expired(current_timestamp)
```

### OS Operations
```gdscript
# Schedule with OS
mock_plugin.os_schedule_notification(id, title, body, delay_seconds)

# Cancel from OS
mock_plugin.os_cancel_notification(id)

# Get scheduled IDs
var ids = mock_plugin.os_get_scheduled_ids()
```

### Test Utilities
```gdscript
# Reset to initial state
mock_plugin.reset()

# Get current state for debugging
var state = mock_plugin.get_database_state()
print(state)  # Shows total, scheduled count, etc.

# Set permission state
mock_plugin.set_permission(false)  # Simulate denied permission
```

## Architecture Notes

### Why Separate Image Storage?

The mock plugin stores images separately (in `_image_blobs` dictionary) to simulate the real plugin behavior:

- **Performance**: Regular queries don't load image data
- **Realism**: Matches actual SQLite schema design
- **Testing**: Validates that image fetching is explicit

### Queue Management Logic

The tests validate the core queue synchronization algorithm:

1. **Get next 24** notifications by timestamp (SQL LIMIT 24)
2. **Cancel OS notifications** not in top 24
3. **Schedule missing notifications** from top 24

This ensures the OS always has the nearest 24 notifications scheduled.

## Troubleshooting

### Tests fail with "NotificationsManager not found"

The tests try to access the global NotificationsManager singleton. This is expected to fail when run in isolation. The mock plugin is self-contained and doesn't require the real manager.

### Permission errors

The mock plugin starts with permissions granted by default. Use `mock_plugin.set_permission(false)` to test permission handling.

### Timestamp issues

Tests use a base timestamp: `Nov 14, 2023 22:13:20 (1700000000)`. All test notifications are relative to this time.
