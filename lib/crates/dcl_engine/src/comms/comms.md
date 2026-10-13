# Decentraland Godot Explorer Communication Architecture

## Overview

The communication system in Decentraland Godot Explorer manages all network communications between players, including movement, chat, profiles, and voice. It uses a centralized message processing architecture that can handle multiple simultaneous room connections.

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
- Manages connection to Archipelago service
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