import { createContext, ReactNode, useContext } from 'react';
import { createCoreDiscoverApi } from './coreApi';
import { publicApi } from './publicApi';
import type { DiscoverApi } from './types';

const ApiContext = createContext<DiscoverApi>(createCoreDiscoverApi(publicApi));

/** Gallery and tests wrap screens with the mock; the app uses the public listings plus the signed calls of the core. */
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
