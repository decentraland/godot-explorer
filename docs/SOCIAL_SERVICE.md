# Social Service - Friends System

The Social Service provides friend requests, friendships, and real-time updates. Access it globally via `Global.social_service`.

## Usage

### Get Friends

```gdscript
var promise = Global.social_service.get_friends(50, 0, 3)
await PromiseUtils.async_awaiter(promise)

if not promise.is_error():
    var friends: Array = promise.get_data()  # Array of address strings
    for friend_address in friends:
        print("Friend: ", friend_address)
```

### Check Friendship Status

```gdscript
var promise = Global.social_service.get_friendship_status("0x123...")
await PromiseUtils.async_awaiter(promise)

var status_data = promise.get_data()  # {status: int, message: string}

match status_data.status:
    3:  # ACCEPTED - Friends
        print("Already friends!")
    7:  # NONE - No relationship
        print("Not friends")
    1:  # REQUEST_RECEIVED - They sent you a request
        print("Pending incoming request")
```

### Send Friend Request

```gdscript
var promise = Global.social_service.send_friend_request("0x123...", "Hi!")
await PromiseUtils.async_awaiter(promise)

if promise.is_error():
    print("Error: ", promise.get_error())
else:
    print("Request sent!")
```

### Accept/Reject Requests

```gdscript
# Get pending requests
var promise = Global.social_service.get_pending_requests(50, 0)
await PromiseUtils.async_awaiter(promise)
var requests = promise.get_data()  # Array of {address, message, created_at}

# Accept
await Global.social_service.accept_friend_request(requests[0].address).on_resolved

# Reject
await Global.social_service.reject_friend_request(requests[1].address).on_resolved
```

### Remove Friend

```gdscript
await Global.social_service.delete_friendship("0x123...").on_resolved
```

## Real-time Updates

Connect to signals to receive live friendship events:

```gdscript
func _ready():
    # Connect to signals
    Global.social_service.friendship_request_received.connect(_on_friend_request)
    Global.social_service.friendship_request_accepted.connect(_on_request_accepted)
    Global.social_service.friendship_deleted.connect(_on_friendship_ended)

func _on_friend_request(address: String, message: String):
    NotificationsManager.show_notification({
        "title": "New Friend Request",
        "body": "From: " + address
    })

func _on_request_accepted(address: String):
    print(address + " accepted your request!")
    # Refresh friends list

func _on_friendship_ended(address: String):
    print("Friendship ended with: " + address)
    # Update UI
```

### Available Signals

| Signal | Parameters | Description |
|--------|-----------|-------------|
| `friendship_request_received` | `(address, message)` | Someone sent you a request |
| `friendship_request_accepted` | `(address)` | Someone accepted your request |
| `friendship_request_rejected` | `(address)` | Someone rejected your request |
| `friendship_deleted` | `(address)` | Friendship was deleted |
| `friendship_request_cancelled` | `(address)` | Someone cancelled their request |

## Friendship Status Codes

| Code | Status | Description |
|------|--------|-------------|
| 0 | REQUEST_SENT | You sent a friend request |
| 1 | REQUEST_RECEIVED | You received a friend request |
| 2 | CANCELED | Request was cancelled |
| 3 | ACCEPTED | Active friendship |
| 4 | REJECTED | Request was rejected |
| 5 | DELETED | Friendship was deleted |
| 7 | NONE | No relationship exists |

## API Reference

### Query Methods

All return `Promise` - must be awaited.

- `get_friends(limit: int, offset: int, status: int) -> Promise<Array[String]>`
  - Returns friend addresses
  - Status: 3 for accepted friends, -1 for all

- `get_pending_requests(limit: int, offset: int) -> Promise<Array[Dictionary]>`
  - Returns: `[{address: String, message: String, created_at: int}, ...]`

- `get_sent_requests(limit: int, offset: int) -> Promise<Array[Dictionary]>`
  - Returns outgoing requests

- `get_friendship_status(address: String) -> Promise<Dictionary>`
  - Returns: `{status: int, message: String}`

- `get_mutual_friends(address: String, limit: int, offset: int) -> Promise<Array[String]>`
  - Returns mutual friend addresses

### Mutation Methods

- `send_friend_request(address: String, message: String) -> Promise<void>`
- `accept_friend_request(address: String) -> Promise<void>`
- `reject_friend_request(address: String) -> Promise<void>`
- `cancel_friend_request(address: String) -> Promise<void>`
- `delete_friendship(address: String) -> Promise<void>`

### Streaming

- `subscribe_to_updates() -> Promise<void>`
  - Automatically called during initialization
  - Enables real-time signals

## Example: Friends List UI

```gdscript
extends Control

var friends: Array = []

func _ready():
    # Connect to real-time updates
    Global.social_service.friendship_request_accepted.connect(_refresh_friends)
    Global.social_service.friendship_deleted.connect(_refresh_friends)

    # Load friends
    await _load_friends()

func _load_friends():
    var promise = Global.social_service.get_friends(100, 0, 3)
    await PromiseUtils.async_awaiter(promise)

    if not promise.is_rejected():
        friends = promise.get_data()
        _update_ui()

func _on_unfriend_button(address: String):
    var promise = Global.social_service.delete_friendship(address)
    await PromiseUtils.async_awaiter(promise)

    if not promise.is_rejected():
        show_success("Friend removed")

func _refresh_friends(_address: String = ""):
    await _load_friends()

func _update_ui():
    # Update your UI with friends array
    pass
```
