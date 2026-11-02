import { useEffect } from 'react';
import { Linking } from 'react-native';
import { useNav } from '../app/NavProvider';
import { core } from '../core';
import { onCoreEvents } from '../core/events';
import {
  Control,
  DisconnectReason,
  EngineNotice_Tags,
  EntryRefusal,
  SceneRequest_Tags,
  SupportModal,
  UpdateDecision,
} from '../core/native';
import type { EngineNotice, SceneRequest } from '../core/native';
import { t } from '../i18n';
import { deviceFacts } from '../platform/deviceFacts';
import { passwordError } from './destination';
import { useToast } from '../ds';
import { useConnectionQuality } from './useConnectionQuality';
import { useModals } from './ModalProvider';
import type { ModalApi } from './ModalProvider';
import type { ModalSpec } from './ModalProvider';

/** The marketplace, which is where the scene's NFT is shown (the in-game NFT card is not ported). */
export const nftUrl = (): string => core().marketplaceBrowseUrl('wearable');

/** Does what the scene asked once the player said yes; answering the scene is separate. */
export function performRequest(request: SceneRequest): void {
  switch (request.tag) {
    case SceneRequest_Tags.OpenExternalUrl:
      Linking.openURL(request.inner.url).catch(() => {});
      return;
    case SceneRequest_Tags.OpenNft:
      Linking.openURL(nftUrl()).catch(() => {});
      return;
    case SceneRequest_Tags.Teleport: {
      const { parcel, realm } = request.inner;
      const state = core().state();
      const here = state.tag === 'InWorld' ? state.inner.realm : undefined;
      if (parcel && (!realm || realm === here)) {
        core().control(new Control.Teleport({ parcel }));
      } else {
        core().enterWorld({ realm, parcel });
      }
      return;
    }
    case SceneRequest_Tags.ChangeRealm:
      core().enterWorld({ realm: request.inner.realm, parcel: undefined });
  }
}

/** The confirmation a scene's request gets: what it asks for and the two answers. */
export function sceneRequestModal(
  id: number,
  request: SceneRequest,
  answer: (id: number, accepted: boolean) => void,
): ModalSpec {
  const buttons = (accept: string, cancel: string) => [
    {
      label: cancel,
      tone: 'secondary' as const,
      onPress: () => answer(id, false),
    },
    { label: accept, onPress: () => answer(id, true) },
  ];
  const base = {
    key: `scene-request-${id}`,
    onDismiss: () => answer(id, false),
  };
  switch (request.tag) {
    case SceneRequest_Tags.OpenExternalUrl:
      return {
        ...base,
        title: t('MODAL_EXTERNAL_LINK_TITLE'),
        body: `${t('MODAL_EXTERNAL_LINK_BODY')}\n${request.inner.url}`,
        buttons: buttons(
          t('MODAL_EXTERNAL_LINK_PRIMARY'),
          t('MODAL_EXTERNAL_LINK_SECONDARY'),
        ),
      };
    case SceneRequest_Tags.OpenNft:
      return {
        ...base,
        title: t('MODAL_EXTERNAL_LINK_TITLE'),
        body: `${t('MODAL_EXTERNAL_LINK_BODY')}\n${request.inner.urn}`,
        buttons: buttons(
          t('MODAL_EXTERNAL_LINK_PRIMARY'),
          t('MODAL_EXTERNAL_LINK_SECONDARY'),
        ),
      };
    case SceneRequest_Tags.Teleport: {
      const { parcel, realm } = request.inner;
      const location = parcel ? `${parcel.x},${parcel.y}` : realm ?? '';
      return {
        ...base,
        title: t('CHAT_TELEPORT_TITLE', { location }),
        body: t('CHAT_TELEPORT_BODY'),
        buttons: buttons(t('CHAT_TELEPORT_GO'), t('CHAT_LINK_CANCEL')),
      };
    }
    case SceneRequest_Tags.ChangeRealm:
      return {
        ...base,
        title: t('CHAT_TELEPORT_TITLE', { location: request.inner.realm }),
        body: request.inner.message ?? t('CHAT_TELEPORT_BODY'),
        buttons: buttons(t('CHAT_TELEPORT_GO'), t('CHAT_LINK_CANCEL')),
      };
  }
}

/** Back to the menus; the core refuses to leave a world the player is not in (a refused entry
 * already took them out), which is not an error here. */
export function leaveWorld(reset: (route: { name: 'home' }) => void): void {
  try {
    core().exitWorld();
  } catch {
    // Not in a world flow: nothing to leave.
  }
  reset({ name: 'home' });
}

/** The disconnect modal the engine withdraws when the connection comes back by itself. */
const RECOVERABLE_KEY = 'disconnected-recoverable';

/** The modals for what the engine tells the player; each one says how to leave or retry. */
export function noticeModal(
  notice: EngineNotice,
  actions: {
    home: () => void;
    reconnect: () => void;
    signOut: () => void;
    askPassword: (realm: string) => void;
  },
): ModalSpec | null {
  const back = (primary: string) => [{ label: primary, onPress: actions.home }];
  switch (notice.tag) {
    case EngineNotice_Tags.EntryRefused: {
      const { reason, realm } = notice.inner;
      if (reason === EntryRefusal.NeedsPassword) {
        actions.askPassword(realm);
        return null;
      }
      if (reason === EntryRefusal.PrivateWorld) {
        return {
          key: 'private-world',
          title: t('MODAL_PRIVATE_WORLD_TITLE', { world: realm }),
          body: t('MODAL_PRIVATE_WORLD_BODY'),
          buttons: back(t('MODAL_PRIVATE_WORLD_PRIMARY')),
        };
      }
      return {
        key: 'invalid-destination',
        title: t('MODAL_INVALID_DESTINATION_TITLE'),
        body: t('MODAL_INVALID_DESTINATION_BODY'),
        buttons: back(t('MODAL_INVALID_DESTINATION_PRIMARY')),
      };
    }
    case EngineNotice_Tags.Banned:
      return {
        key: 'banned',
        title: t('MODAL_BAN_KICKED_TITLE'),
        body: t('MODAL_BAN_KICKED_BODY'),
        dismissable: false,
        urgent: true,
        buttons: back(t('MODAL_BAN_KICKED_PRIMARY')),
      };
    case EngineNotice_Tags.DisconnectRecovered:
      return null;
    case EngineNotice_Tags.LowMemory:
      return {
        key: 'low-memory',
        title: t('MODAL_LOW_MEMORY_TITLE'),
        body: t('MODAL_LOW_MEMORY_BODY'),
        buttons: [
          {
            label: t('MODAL_LOW_MEMORY_SECONDARY'),
            tone: 'secondary',
            onPress: actions.home,
          },
          { label: t('MODAL_LOW_MEMORY_PRIMARY') },
        ],
      };
    case EngineNotice_Tags.Disconnected: {
      const { reason } = notice.inner;
      if (
        reason === DisconnectReason.DuplicateSession ||
        reason === DisconnectReason.PlatformBan
      ) {
        const banned = reason === DisconnectReason.PlatformBan;
        return {
          key: 'disconnected',
          title: banned
            ? t('MODAL_BAN_KICKED_TITLE')
            : t('MODAL_SESSION_ENDED_DUPLICATE_TITLE'),
          body: banned
            ? t('MODAL_BAN_KICKED_BODY')
            : t('MODAL_SESSION_ENDED_DUPLICATE_BODY'),
          dismissable: false,
          urgent: true,
          buttons: [
            {
              label: t('MODAL_SESSION_ENDED_SECONDARY'),
              tone: 'secondary',
              onPress: actions.home,
            },
            {
              label: t('MODAL_SESSION_ENDED_SIGN_IN_PRIMARY'),
              onPress: actions.signOut,
            },
          ],
        };
      }
      const closed = reason === DisconnectReason.RoomClosed;
      return {
        // Only this one closes by itself when the connection comes back.
        key: closed ? 'disconnected' : RECOVERABLE_KEY,
        title: closed
          ? t('MODAL_SESSION_ENDED_ROOM_CLOSED_TITLE')
          : t('MODAL_SESSION_ENDED_OTHER_TITLE'),
        body: closed
          ? t('MODAL_SESSION_ENDED_ROOM_CLOSED_BODY')
          : t('MODAL_SESSION_ENDED_OTHER_BODY'),
        dismissable: false,
        urgent: true,
        buttons: [
          {
            label: t('MODAL_SESSION_ENDED_SECONDARY'),
            tone: 'secondary',
            onPress: actions.home,
          },
          {
            label: t('MODAL_SESSION_ENDED_RETRY_PRIMARY'),
            onPress: actions.reconnect,
          },
        ],
      };
    }
  }
}

/** Asks for the password of a protected world and enters it when the world accepts it. */
export function askWorldPassword(modals: ModalApi, realm: string): void {
  modals.askPassword({
    title: t('MODAL_WORLD_PASSWORD_TITLE'),
    subtitle: t('MODAL_WORLD_PASSWORD_SUBTITLE'),
    verify: async password => {
      const answer = await core().resolveDestinationWithSecret(realm, password);
      const error = passwordError(answer);
      if (error === null) {
        core().control(new Control.SetWorldSecret({ realm, secret: password }));
      }
      return error;
    },
    onCancel: () => {},
    onAccepted: () => core().enterWorld({ realm, parcel: undefined }),
  });
}

/** Raises the modals the core and the engine ask for: scene requests, a crashed scene and, once
 * the player is in, the update and device-support notices. */
export function CoreModals({ ready }: { ready: boolean }) {
  const modals = useModals();
  const { reset } = useNav();
  const toast = useToast();
  useConnectionQuality(ready, modals, toast, () => leaveWorld(reset));

  useEffect(
    () =>
      onCoreEvents({
        sceneRequest: (id, request) =>
          modals.show(
            sceneRequestModal(id, request, (requestId, accepted) => {
              if (accepted) {
                performRequest(request);
              }
              core().control(
                new Control.AnswerSceneRequest({ id: requestId, accepted }),
              );
            }),
          ),
        engineNotice: notice => {
          const leave = () => leaveWorld(reset);
          if (notice.tag === EngineNotice_Tags.EntryRefused) {
            reset({ name: 'home' });
          }
          if (notice.tag === EngineNotice_Tags.DisconnectRecovered) {
            modals.dismiss(RECOVERABLE_KEY);
            return;
          }
          const modal = noticeModal(notice, {
            home: leave,
            reconnect: () => core().control(new Control.Reconnect()),
            signOut: () => core().signOut(),
            askPassword: realm => askWorldPassword(modals, realm),
          });
          if (modal) {
            modals.show(modal);
          }
        },
        sceneCrashed: () => {
          const state = core().state();
          modals.show({
            key: 'scene-crash',
            title: t('MODAL_SCENE_CRASH_TITLE'),
            body: t('MODAL_SCENE_CRASH_BODY'),
            dismissable: false,
            urgent: true,
            buttons: [
              {
                label: t('MODAL_SCENE_CRASH_SECONDARY'),
                tone: 'secondary',
                onPress: () => leaveWorld(reset),
              },
              {
                label: t('MODAL_SCENE_CRASH_PRIMARY'),
                onPress: () => {
                  if (state.tag === 'InWorld') {
                    const { realm, parcel } = state.inner;
                    core().exitWorld();
                    core().enterWorld({ realm, parcel });
                  }
                },
              },
            ],
          });
        },
      }),
    [modals, reset],
  );

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;
    (async () => {
      const gate = await core()
        .versionGate()
        .catch(() => null);
      if (cancelled || !gate || gate.decision === UpdateDecision.Proceed) {
        return;
      }
      const required = gate.decision === UpdateDecision.Required;
      modals.show({
        key: 'update',
        title: t('AUTH_UPDATE_TITLE'),
        body: t('AUTH_UPDATE_BODY'),
        dismissable: !required,
        buttons: [
          ...(required
            ? []
            : [
                {
                  label: t('COMMON_CANCEL'),
                  tone: 'secondary' as const,
                  onPress: () => core().snoozeVersionGate(),
                },
              ]),
          {
            label: t('COMMON_UPDATE'),
            onPress: () => {
              Linking.openURL(
                'market://details?id=org.decentraland.godotexplorer',
              ).catch(() => {});
            },
          },
        ],
      });
    })();
    (async () => {
      const support = await deviceFacts()
        .then(facts => core().deviceSupport(facts))
        .catch(() => null);
      if (cancelled || !support || support.modal === undefined) {
        return;
      }
      const end = support.modal === SupportModal.EndOfSupport;
      core().deviceSupportModalShown(support.modal);
      modals.show({
        key: 'device-support',
        title: end
          ? t('MODAL_END_OF_DEVICE_SUPPORT_TITLE')
          : t('NOTIFICATIONS_LIMITED_PERFORMANCE'),
        body: end
          ? t('MODAL_END_OF_DEVICE_SUPPORT_BODY')
          : t('MODAL_BELOW_MINSPEC_BODY'),
        buttons: [{ label: t('MODAL_END_OF_DEVICE_SUPPORT_PRIMARY') }],
      });
    })();
    return () => {
      cancelled = true;
    };
  }, [ready, modals]);

  return null;
}
