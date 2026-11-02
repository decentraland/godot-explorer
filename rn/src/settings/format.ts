import { t } from '../i18n';

const MB = 1e6;
const GB = 1e9;

/** Zero and small sizes read in MB ("0 MB"), from 0.1 GB up in GB with one decimal ("0.5 GB"). */
export function formatCacheSize(bytes: number): string {
  if (bytes >= 0.1 * GB) {
    return t('SETTINGS_CACHE_SIZE_GB', { size: (bytes / GB).toFixed(1) });
  }
  return t('SETTINGS_CACHE_SIZE_MB', {
    size: Math.round(Math.max(0, bytes) / MB),
  });
}

export const formatMaxCache = (gb: number): string =>
  t('SETTINGS_CACHE_SIZE_GB', { size: gb });

/** Share of the cap in use, 0..1. */
export function cacheFraction(bytes: number, maxGb: number): number {
  return maxGb > 0 ? Math.min(1, Math.max(0, bytes / (maxGb * GB))) : 0;
}

export function shortenAddress(address: string): string {
  return address.length > 12
    ? `${address.slice(0, 5)}…${address.slice(-5)}`
    : address;
}

/** Split layout needs a wide window; portrait phones stay master-detail. */
export const isSplitLayout = (width: number, height: number): boolean =>
  width > height && width >= 560;
