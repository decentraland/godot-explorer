import { useEffect, useReducer, useRef, useState } from 'react';
import { t } from '../i18n';
import type { HudApi } from './hudApi';
import { chatFeedReducer, initialChatFeed } from './chatFeed';
import type { ChatProfile } from './types';

export const fullName = (p: ChatProfile): string => `${p.name}${p.tag}`;

/** Chat history, unread count and the profiles of everyone who spoke, fed by the HUD api. */
export function useChatFeed(api: HudApi, chatOpen: boolean, welcome = true) {
  const [feed, dispatch] = useReducer(chatFeedReducer, initialChatFeed);
  const [profiles, setProfiles] = useState<Record<string, ChatProfile>>({});
  const [own, setOwn] = useState<ChatProfile | null>(null);
  const open = useRef(chatOpen);
  const seq = useRef(0);
  open.current = chatOpen;

  useEffect(() => {
    let alive = true;
    const remember = (profile: ChatProfile) =>
      alive &&
      setProfiles(prev =>
        prev[profile.address] ? prev : { ...prev, [profile.address]: profile },
      );
    api
      .ownProfile()
      .then(profile => {
        if (alive) {
          setOwn(profile);
          remember(profile);
        }
      })
      .catch(() => {});
    if (welcome) {
      dispatch({
        type: 'line',
        chatOpen: false,
        line: {
          id: 'welcome',
          address: 'system',
          message: t('CHAT_SYSTEM_WELCOME'),
          timestamp: Date.now(),
        },
      });
    }
    const stop = api.subscribe(event => {
      if (event.type === 'chat') {
        seq.current += 1;
        const { address, message, timestamp, profile } = event.data;
        dispatch({
          type: 'line',
          chatOpen: open.current,
          line: {
            id: `${timestamp}-${seq.current}`,
            address,
            message,
            timestamp,
          },
        });
        if (profile) {
          remember(profile);
        } else if (address !== 'system') {
          api
            .resolveProfile(address)
            .then(remember)
            .catch(() => {});
        }
      } else if (event.type === 'unread_chat') {
        dispatch({ type: 'unread', count: event.data.count });
      }
    });
    return () => {
      alive = false;
      stop();
    };
  }, [api, welcome]);

  return { feed, profiles, own, markRead: () => dispatch({ type: 'read' }) };
}
