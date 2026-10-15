import { core } from './index';
import type {
  AppListener,
  AvatarState,
  ChatLine,
  EngineNotice,
  HudSnapshot,
  LoadingDetail,
  OpenUrl,
  ReportPayload,
  SceneRequest,
  SettingEntry,
  State,
  UiState,
} from './native';

export type CoreHandlers = Partial<AppListener>;

const handlers = new Set<CoreHandlers>();
let registered = false;

/** The one native listener fans out to every subscriber, so screens never register their own. */
function ensureRegistered() {
  if (registered) {
    return;
  }
  registered = true;
  core().addListener({
    stateChanged: (state: State) =>
      handlers.forEach(h => h.stateChanged?.(state)),
    sceneCrashed: () => handlers.forEach(h => h.sceneCrashed?.()),
    authError: (code: string) => handlers.forEach(h => h.authError?.(code)),
    openUrl: (request: OpenUrl) => handlers.forEach(h => h.openUrl?.(request)),
    settingsChanged: (entry: SettingEntry) =>
      handlers.forEach(h => h.settingsChanged?.(entry)),
    hud: (snapshot: HudSnapshot) => handlers.forEach(h => h.hud?.(snapshot)),
    chat: (line: ChatLine) => handlers.forEach(h => h.chat?.(line)),
    unreadChat: (count: number) => handlers.forEach(h => h.unreadChat?.(count)),
    uiState: (state: UiState) => handlers.forEach(h => h.uiState?.(state)),
    openBackpack: (emotes: boolean) =>
      handlers.forEach(h => h.openBackpack?.(emotes)),
    loadingDetail: (detail: LoadingDetail) =>
      handlers.forEach(h => h.loadingDetail?.(detail)),
    avatarChanged: (avatar: AvatarState) =>
      handlers.forEach(h => h.avatarChanged?.(avatar)),
    sceneRequest: (id: number, request: SceneRequest) =>
      handlers.forEach(h => h.sceneRequest?.(id, request)),
    engineNotice: (notice: EngineNotice) =>
      handlers.forEach(h => h.engineNotice?.(notice)),
    report: (payload: ReportPayload) =>
      handlers.forEach(h => h.report?.(payload)),
    reportTag: (key: string, value: string | undefined) =>
      handlers.forEach(h => h.reportTag?.(key, value)),
  });
}

/** Subscribes to the core's events; returns the unsubscribe function. */
export function onCoreEvents(subscriber: CoreHandlers): () => void {
  ensureRegistered();
  handlers.add(subscriber);
  return () => {
    handlers.delete(subscriber);
  };
}

export function resetCoreEventsForTests() {
  handlers.clear();
  registered = false;
}
