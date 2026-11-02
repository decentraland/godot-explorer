import { useEffect, useSyncExternalStore } from 'react';
import { getLocale, setLocale, subscribeLocale } from '../i18n';
import { getSettingsApi } from '../settings/api';

/** Keeps the i18n locale equal to the `locale` setting; returns it so the caller can remount on change. */
export function useLocaleSync() {
  useEffect(() => {
    const api = getSettingsApi();
    const apply = () =>
      api
        .getAll()
        .then(values => setLocale(values.locale))
        .catch(() => {});
    apply();
    return api.subscribe(apply);
  }, []);
  return useSyncExternalStore(subscribeLocale, getLocale);
}
