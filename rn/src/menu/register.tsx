import { useCallback } from 'react';
import { registerScreen } from '../app/registry';
import { useNav } from '../app/NavProvider';
import { MenuDrawer, Navigate } from './MenuDrawer';
import './routes';

function WorldDrawerSlot() {
  const nav = useNav();
  const navigate = useCallback<Navigate>(
    page => nav.push({ name: page }),
    [nav],
  );
  return <MenuDrawer navigate={navigate} />;
}

registerScreen('worldDrawer', WorldDrawerSlot);
