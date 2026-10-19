# Decentraland Godot Explorer Communication Architecture

## Overview

The communication system in Decentraland Godot Explorer manages all network communications between players, including movement, chat, profiles, and voice. It uses a centralized message processing architecture that can handle multiple simultaneous room connections.

## Pulse session ownership

Pulse is required for normal multiplayer. Remote `pulse` and `dual-channel` flags do not
control activation or restore the retired LiveKit avatar fallback. Movement and emotes use
Pulse; `--livekit-movement` / `dual-channel=true` explicitly duplicate them for development.
`--no-pulse` / `pulse=false` means no island multiplayer. Local previews remain opt-in via
`--pulse`, an explicit Pulse endpoint, or an explicit Pulse realm.

The WS Connector still delivers island assignments, but only while Pulse is established.
Losing Pulse drops the island and its cached token. After Pulse recovers, the connector
re-handshakes to obtain a fresh assignment from gatekeeper. It retries the handshake if no
assignment arrives, including when gatekeeper still sees a closing participant.

- Retryable Pulse failures back off from 2 seconds to a 30-second cap. Transport connection
  attempts time out after 10 seconds. A 10-second observed outage shows the existing Retry
  modal without stopping transport recovery; establishment clears the outage modal.
- Island `ParticipantRemoved` discards the token and waits 10 seconds for Pulse's reason.
  If Pulse survives, a fresh WS handshake requests a new assignment; the debug panel shows
  `removed_waiting_for_pulse` during the grace period.
- Pulse `DuplicateSession` stops all automatic comms recovery and shows Session Ended
  (Sign in / Exit). Pulse `Banned` shows a terminal ban modal
  (Sign in / Exit), separate from scene-ban navigation. Other terminal Pulse failures require
  explicit Retry rather than an automatic loop.
- Resume grace lasts 30 seconds and permits one Pulse session reclaim. Repeated duplicate
  eviction cannot continuously reclaim the account during that window.
- Cleanup replaces the scene-room result channel, invalidating pending requests from the
  previous session. Loading, flag callbacks, and delayed retries cannot reopen an ended session.

### Validation

Automated checks:

```sh
cargo run -- test-gdscript
cargo test --manifest-path lib/Cargo.toml --lib comms::
cargo clippy --manifest-path lib/Cargo.toml --lib -- -D warnings
```

Headless UI tests are `godot/src/test/comms/test_disconnect_handler.gd` and
`test_disconnect_modal.gd`. They load the production scripts with fake services and test
retry cancellation, deferred modals, recovery, and sign-in without connecting to servers.
Both suites are discovered by the standard test command above.

Device acceptance checks (require a running client and the migrated backend):

1. Start with `--pulse-server 127.0.0.1:65534` targeting an unused UDP port: Retry appears,
   Pulse keeps retrying, and no island joins occur. Restore a reachable endpoint and confirm recovery.
2. On zone, record the resolved realm provider, Pulse endpoint, and WS Connector endpoint
   before testing; do not substitute a legacy catalyst Archipelago endpoint.
3. Sign the same wallet into two devices: the older session shows Session Ended and stops
   reconnecting. Verify both island-first and Pulse-first disconnect ordering in logs.
4. Background for 10 seconds and resume; briefly enable airplane mode for 5 seconds.
   Peers should recover without a modal or repeated island token rejection.
5. Verify normal login, realm teleport, scene voice, and bans. Run on Android and iOS.

## Architecture Components

### 1. CommunicationManager (communication_manager.rs)
The central orchestrator for all communications. It manages:
- Connection lifecycle
- Message routing between rooms
- Profile broadcasting
- A shared `MessageProcessor` instance

Key responsibilities:
- Creates and maintains a single `MessageProcessor` instance
- Manages multiple room connections (main room + scene rooms)
- Routes outgoing messages to all active rooms
- Handles connection protocol negotiation

### 2. MessageProcessor (adapter/message_processor.rs)
The heart of the message processing system. A single instance handles ALL incoming messages from ALL rooms:
- Manages peer lifecycle (creation, updates, removal)
- Processes RFC4 protocol messages (movement, chat, profiles, etc.)
- Handles avatar creation/removal
- Manages profile fetching and caching
- Prevents memory exhaustion with bounded queues

Key features:
- Uses channels to receive messages from multiple rooms asynchronously
- Maintains a unified view of all peers across all rooms
- Handles peer timeouts and cleanup
- Manages profile requests and responses

### 3. Room Adapters

#### WebSocketRoom (adapter/ws_room.rs)
- Direct WebSocket connection to a room
- Implements the Adapter trait
- Sends messages to the shared MessageProcessor

#### LivekitRoom (adapter/livekit.rs)
- LiveKit-based room for WebRTC communications
- Supports voice chat
- Can be configured with `auto_subscribe`:
  - `true` (default): Automatically receives all peers (used for main rooms)
  - `false`: Manual subscription control (used for scene rooms)

#### ArchipelagoManager (adapter/archipelago.rs)
- Maintains the supported WS Connector protocol for Pulse-owned island assignments
- Handles authentication and island changes
- Creates LiveKit rooms dynamically when moving between islands
- Always uses the shared MessageProcessor (no local processing)

### 4. Adapter Trait (adapter/adapter_trait.rs)
Common interface for all room types:
```rust
pub trait Adapter {
    fn poll(&mut self) -> bool;
    fn clean(&mut self);
    fn consume_chats(&mut self) -> Vec<(H160, rfc4::Chat)>;
    fn consume_scene_messages(&mut self, scene_id: &str) -> Vec<(H160, Vec<u8>)>;
    fn change_profile(&mut self, new_profile: UserProfile);
    fn send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool;
    fn broadcast_voice(&mut self, frame: Vec<i16>);
    fn support_voice_chat(&self) -> bool;
}
```

## Message Flow

### Incoming Messages
1. Room adapter receives a message from the network
2. Room adapter creates an `IncomingMessage` with:
   - Message content
   - Sender's address
   - Room ID (for tracking which room it came from)
3. Room adapter sends the message through its channel to MessageProcessor
4. MessageProcessor:
   - Updates peer state
   - Processes the message based on type
   - Updates avatars if needed
   - Queues responses if needed

### Outgoing Messages
1. MessageProcessor queues outgoing messages (e.g., ProfileResponse)
2. CommunicationManager consumes outgoing messages during its poll
3. CommunicationManager broadcasts to ALL active rooms:
   - Main room (WebSocket or LiveKit)
   - Scene room (LiveKit)
   - Archipelago's adapter (if connected)

### Profile Broadcasting
- ProfileVersion packets are broadcast every 10 seconds
- Profile changes trigger immediate broadcasts
- Broadcasts go to all active rooms simultaneously

## Connection Types

### 1. Direct Connections
- **ws-room**: WebSocket room connection
- **livekit**: Direct LiveKit connection
- **offline**: No connection

### 2. Archipelago Connection
- Connects to Archipelago service via WebSocket
- Receives island change notifications
- Dynamically creates LiveKit rooms for each island
- Maintains position heartbeat to Archipelago

### 3. Scene Rooms
- Created when entering a new scene
- Uses LiveKit with manual subscription control
- Allows proximity-based voice chat and data exchange
- Managed separately from the main room connection

## Key Design Decisions

### Centralized Message Processing
- Single MessageProcessor instance handles all rooms
- Prevents duplicate processing
- Maintains consistent peer state across rooms
- Simplifies avatar management

### Channel-Based Communication
- Asynchronous message passing between rooms and processor
- Prevents blocking on network operations
- Allows concurrent message handling

### Room Identification
- Each message includes the room ID it came from
- Allows tracking peer activity per room
- Enables proper cleanup when peers leave specific rooms

### Bounded Queues
- Prevents memory exhaustion under load
- Configurable limits for different message types
- Drops oldest messages when limits reached

## Lifecycle Management

### Peer Lifecycle
1. **Join**: First message from a new address creates a peer
2. **Update**: Activity tracked per room
3. **Leave**: Explicit leave message or timeout removes peer from room
4. **Cleanup**: Peer removed when no longer in any room

### Connection Lifecycle
1. **Initialization**: CommunicationManager creates shared MessageProcessor
2. **Connection**: Room adapters connect to MessageProcessor via channels
3. **Operation**: Messages flow through channels to processor
4. **Cleanup**: Rooms cleaned up, processor reset if needed

## Error Handling

- Network errors handled at adapter level
- Processing errors logged but don't crash the system
- Failed profile fetches retry with exponential backoff
- Connection failures trigger reconnection attempts

## Performance Considerations

- Message processing is non-blocking
- Channels have size limits to prevent unbounded growth
- Inactive peer cleanup prevents memory leaks
- Profile caching reduces network requests
- Movement compression reduces bandwidth usage

## Future Extensibility

The architecture supports:
- Adding new room types (implement Adapter trait)
- New message types (extend MessageType enum)
- Additional protocols (extend connection handling)
- Custom room selection strategies
- Enhanced proximity algorithms for scene rooms