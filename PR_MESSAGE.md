# Friends Frontend Implementation

This PR introduces the initial frontend implementation for the friends system, including new UI components, integration with the main HUD, and the foundation for social interactions.

## 🎨 New Components

### AnimatedButton Component
Created a new `BaseAnimatedButton` abstract class that extends `Button` to provide a reusable animated button component with the following features:
- **Animated sprite support**: Uses `AnimatedSprite2D` with configurable sprite frames and scale
- **Badge system**: Displays unread count badges with automatic formatting (shows "99+" for counts over 99)
- **Toggle animations**: Smooth animations when opening/closing associated panels using tweens with ease-in/ease-out transitions
- **Haptic feedback**: Provides subtle vibration feedback on mobile devices
- **Metrics tracking**: Integrated with the analytics system for button click tracking

This component serves as the base for both the notifications and friends buttons, ensuring consistent behavior and visual feedback across the HUD.

### Social Item Component
Created a reusable `social_item` component that can display different types of social interactions:
- **Multiple display modes**: Supports ONLINE, OFFLINE, REQUEST, NEARBY, and BLOCKED item types
- **Avatar integration**: Displays profile pictures, nicknames, and avatar status
- **Interactive elements**: Includes buttons for adding friends, muting users, and blocking (with visibility controlled by item type)
- **Dynamic styling**: Adapts its layout and visible elements based on the social type
- **Profile integration**: Allows opening user profiles on interaction

### Social List Component
Implemented a `social_list` component that manages different types of player lists:
- **Dynamic list management**: Automatically adds/removes items as avatars join or leave the scene
- **Multiple list types**: Handles ONLINE, OFFLINE, REQUEST, NEARBY, and BLOCKED lists
- **Real-time updates**: Connects to avatar scene changes to keep lists synchronized
- **Sorting**: Automatically sorts items alphabetically by avatar name
- **Signal-based architecture**: Emits size change signals for UI updates

## 🎯 Main HUD Integration

### Friends Button
- Added a new friends button to the main HUD, positioned alongside the notifications button
- Uses the `BaseAnimatedButton` component for consistent behavior and animations
- Integrated with the HUD's focus management system (releases focus when opened, restores when closed)
- Includes proper mobile touch handling and camera control management

### Friends Panel
- Created a comprehensive friends panel with tabbed interface:
  - **Friends tab**: Displays online and offline friends with collapsible sections
  - **Nearby tab**: Shows players currently in the scene (moved from chat)
  - **Blocked tab**: Lists blocked users
- **Request management**: Includes a collapsible section for pending friend requests with visual indicators
- **Empty states**: Displays helpful messages when lists are empty
- **Responsive design**: Properly handles touch events and prevents camera rotation when panel is open

## 🔄 UI/UX Improvements

### Removed Nearby Players from Chat
- Removed the nearby players list from the chat interface
- Nearby players are now accessible through the dedicated Friends panel, providing better organization and separation of concerns
- This change improves the chat interface by focusing it solely on messaging functionality

## 🔌 Backend Integration (WIP)

### FriendsService Communication
- **Foundation laid**: Code structure prepared for integration with FriendsService
- **Signal connections**: Placeholder methods ready for connecting to friends manager signals
- **Unread count tracking**: Badge system ready to display friend request counts once backend is connected
- **Status**: Integration with FriendsService is in progress and will be completed in a follow-up PR

## 🎨 Styling & Layout (WIP)

- **Initial styling**: Basic styling and layout implemented for all new components
- **Visual consistency**: Components follow the existing design system
- **Polish in progress**: Further refinement of spacing, colors, and animations is ongoing
- **Responsive behavior**: Mobile and desktop layouts are functional but may require additional tweaks

## 📝 Technical Details

- All new components follow Godot's node-based architecture
- Proper signal-based communication between components
- Memory-efficient list management with automatic cleanup
- Integration with existing Global systems (avatars, identity, metrics)
- Support for both mobile and desktop platforms

## 🚧 Known Limitations / TODO

- FriendsService backend integration pending
- Additional styling refinements needed
- Friend request actions (accept/decline) to be implemented
- Block/unblock functionality UI to be finalized
- Profile picture loading states may need enhancement

---

**Note**: This PR focuses on the frontend implementation. Backend integration and final polish will be addressed in subsequent PRs.

