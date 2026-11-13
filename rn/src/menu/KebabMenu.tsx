import { IconName, Menu } from '@dcl/ui-native';
import { t } from '../i18n';

export type KebabAction = 'share' | 'reportContent' | 'reportBug';

const ITEMS: { action: KebabAction; icon: IconName; label: string }[] = [
  { action: 'share', icon: 'share', label: 'MENU_KEBAB_SHARE' },
  { action: 'reportContent', icon: 'flag', label: 'MENU_KEBAB_REPORT_CONTENT' },
  { action: 'reportBug', icon: 'bug', label: 'MENU_KEBAB_REPORT_BUG' },
];

/** Popover under the header's three dots; the caller dismisses it on an outside tap. */
export function KebabMenu({
  onSelect,
}: {
  onSelect: (action: KebabAction) => void;
}) {
  return (
    <Menu
      items={ITEMS.map(item => ({
        key: item.action,
        icon: item.icon,
        label: t(item.label as 'MENU_KEBAB_SHARE'),
      }))}
      onSelect={onSelect}
    />
  );
}
