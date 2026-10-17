import { useEffect, useRef } from 'react';
import { getMenuApi } from './api';
import { useDrawerState } from './drawerState';
import { unreadCount } from './notifications';
import { useLoad } from './useLoad';

/** True while a notification is unread; refreshes each time the drawer closes. For the HUD menu button's badge. */
export function useMenuUnread(): boolean {
  const { open } = useDrawerState();
  const { state, reload } = useLoad(() => getMenuApi().notifications());
  const wasOpen = useRef(false);
  useEffect(() => {
    if (wasOpen.current && !open) {
      reload();
    }
    wasOpen.current = open;
  }, [open, reload]);
  return (
    state.status === 'ready' &&
    state.data.status === 'ok' &&
    unreadCount(state.data.items) > 0
  );
}
