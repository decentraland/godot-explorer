import { Menu } from '@dcl/ui-native';
import { t } from '../i18n';
import { ItemFilter } from './selection';

const OPTIONS: {
  key: ItemFilter;
  label: 'BACKPACK_ALL' | 'BACKPACK_COLLECTIBLES';
}[] = [
  { key: 'all', label: 'BACKPACK_ALL' },
  { key: 'collectibles', label: 'BACKPACK_COLLECTIBLES' },
];

/** Dropdown under the filter button: the DS `Menu` with a check on the active filter. */
export function FilterMenu({
  value,
  onChange,
}: {
  value: ItemFilter;
  onChange: (next: ItemFilter) => void;
}) {
  return (
    <Menu
      items={OPTIONS.map(option => ({
        key: option.key,
        label: t(option.label),
        selected: option.key === value,
        check: option.key === value,
      }))}
      onSelect={onChange}
    />
  );
}
