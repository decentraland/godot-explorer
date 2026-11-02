/** Copy of the dialogs the player meets: scene requests, entry refusals, disconnects, update and device notices. */
export const modals = {
  NOTIFICATIONS_LIMITED_PERFORMANCE: 'Limited performance',
  AUTH_UPDATE_BODY:
    'A new version of Decentraland is available. Update for the best experience.',
  AUTH_UPDATE_TITLE: 'New Version Available',
  COMMON_UPDATE: 'UPDATE',
  MODAL_BAN_KICKED_BODY: 'Please contact support for more information.',
  MODAL_BAN_KICKED_PRIMARY: 'BACK TO DISCOVER',
  MODAL_BAN_KICKED_TITLE: "You've been banned",
  MODAL_BAN_PRE_CHECK_BODY:
    "You're banned from this scene.\nPlease contact support for more information.",
  MODAL_BAN_PRE_CHECK_PRIMARY: 'BACK TO DISCOVER',
  MODAL_BAN_PRE_CHECK_TITLE: "You can't enter",
  MODAL_BELOW_MINSPEC_BODY:
    'Your device is below our recommended specs. You may notice slow-downs, crashes or heating issues while playing.',
  MODAL_CONNECTION_LOST_BODY:
    'Please check your internet connection and try again.',
  MODAL_CONNECTION_LOST_PRIMARY: 'RETRY',
  MODAL_CONNECTION_LOST_SECONDARY: 'EXIT',
  MODAL_CONNECTION_LOST_TITLE: 'Connection lost',
  MODAL_END_OF_DEVICE_SUPPORT_BODY:
    "Decentraland will no longer receive updates on this device. You can keep using the current version, but you won't be able to reinstall it if you uninstall.",
  MODAL_END_OF_DEVICE_SUPPORT_PRIMARY: 'OK',
  MODAL_EXTERNAL_LINK_BODY:
    "You're about to visit an external website. Make sure you trust this site before continuing.",
  MODAL_EXTERNAL_LINK_PRIMARY: 'OPEN LINK',
  MODAL_EXTERNAL_LINK_SECONDARY: 'CANCEL',
  MODAL_EXTERNAL_LINK_TITLE: 'Open external link?',
  MODAL_INVALID_DESTINATION_BODY:
    'This place may have been removed or is no longer available.',
  MODAL_INVALID_DESTINATION_PRIMARY: 'OK',
  MODAL_INVALID_DESTINATION_TITLE: 'Place not found',
  MODAL_LOW_MEMORY_BODY:
    'This place may not run smoothly on your device and could close unexpectedly.',
  MODAL_LOW_MEMORY_PRIMARY: 'CONTINUE',
  MODAL_LOW_MEMORY_SECONDARY: 'BACK TO DISCOVER',
  MODAL_LOW_MEMORY_TITLE: 'Low memory',
  MODAL_PRIVATE_WORLD_BODY: 'Only invited people can enter.',
  MODAL_PRIVATE_WORLD_PRIMARY: 'OK',
  MODAL_PRIVATE_WORLD_TITLE: '{world} is private',
  MODAL_SCENE_CRASH_BODY:
    'This scene stopped working. Please reload or go back to discover.',
  MODAL_SCENE_CRASH_PRIMARY: 'RELOAD',
  MODAL_SCENE_CRASH_SECONDARY: 'BACK',
  MODAL_SCENE_CRASH_TITLE: 'Scene error',
  MODAL_SESSION_ENDED_DUPLICATE_BODY:
    'Your session was ended because your account logged in from another location.',
  MODAL_SESSION_ENDED_DUPLICATE_TITLE: 'Session Ended',
  MODAL_SESSION_ENDED_OTHER_BODY:
    'You have been disconnected from the server. Please try again later.',
  MODAL_SESSION_ENDED_OTHER_TITLE: 'Disconnected',
  MODAL_SESSION_ENDED_RETRY_PRIMARY: 'RECONNECT',
  MODAL_SESSION_ENDED_ROOM_CLOSED_BODY: 'The room you were in has been closed.',
  MODAL_SESSION_ENDED_ROOM_CLOSED_TITLE: 'Room Closed',
  MODAL_SESSION_ENDED_SECONDARY: 'EXIT',
  MODAL_SESSION_ENDED_SIGN_IN_PRIMARY: 'SIGN IN',
  MODAL_WORLD_PASSWORD_BUSY:
    "Couldn't check that password right now. Try again in a moment.",
  MODAL_WORLD_PASSWORD_CONFIRM: 'ENTER',
  MODAL_WORLD_PASSWORD_PLACEHOLDER: 'Password',
  MODAL_WORLD_PASSWORD_SUBTITLE:
    'This World is protected. Enter its password to go in.',
  MODAL_WORLD_PASSWORD_TITLE: 'Password required',
  MODAL_WORLD_PASSWORD_TOO_MANY:
    'Too many attempts. Wait a minute and try again.',
  MODAL_WORLD_PASSWORD_WRONG: "That password didn't work. Try again.",
  TOAST_POOR_CONNECTION_BODY:
    'Your connection is unstable. Some features may not work properly.',
  TOAST_POOR_CONNECTION_TITLE: 'Poor connection',
};
