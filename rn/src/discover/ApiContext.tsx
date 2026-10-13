import { createContext, ReactNode, useContext } from 'react';
import { godotAvailable } from '../GodotBridge';
import { createBridgeDiscoverApi } from './bridgeApi';
import { publicApi } from './publicApi';
import type { DiscoverApi } from './types';

const ApiContext = createContext<DiscoverApi>(
  godotAvailable ? createBridgeDiscoverApi(publicApi) : publicApi,
);

/** Gallery and tests wrap screens with the mock; the app uses the bridge-backed API with the engine and the public one without. */
export function ApiProvider({
  api,
  children,
}: {
  api: DiscoverApi;
  children: ReactNode;
}) {
  return <ApiContext.Provider value={api}>{children}</ApiContext.Provider>;
}

export const useApi = (): DiscoverApi => useContext(ApiContext);
