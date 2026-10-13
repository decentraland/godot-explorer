import { ComponentProps } from 'react';
import { ActionSquareButton } from './ActionSquareButton';
import { BottomSheet } from './BottomSheet';
import { Chip, StatusPill } from './StatusPill';
import { CollapsibleSection } from './CollapsibleSection';
import { Dialog } from './Dialog';
import { Dropdown } from './Dropdown';
import { EmptyState } from './EmptyState';
import { HudCircleButton } from './HudCircleButton';
import { LinkRow } from './LinkRow';
import { OutlinedButton } from './OutlinedButton';
import { PageDots, SnapCarousel } from './Carousel';
import { ProgressBar } from './ProgressBar';
import { ReadOnlyField } from './ReadOnlyField';
import { SearchField } from './SearchField';
import { Skeleton } from './Skeleton';
import { Slider } from './Slider';
import { Switch } from './Switch';
import { TextField } from './TextField';
import { ToastBubble } from './Toast';
import { WarningBanner } from './WarningBanner';

const noop = () => {};

/** One small example props object per new component, for the gallery. Controlled values are static. */
export const demos = {
  Switch: {
    value: true,
    label: 'Dynamic Graphics',
    onChange: noop,
  } satisfies ComponentProps<typeof Switch>,
  Slider: {
    label: 'General',
    value: 70,
    onChange: noop,
  } satisfies ComponentProps<typeof Slider>,
  Dropdown: {
    label: 'App Language',
    value: 'en',
    options: [
      { value: 'en', label: 'English' },
      { value: 'es', label: 'Espanol' },
      { value: 'pt', label: 'Portugues (Brasil)' },
    ],
    helper: 'Changes the Decentraland interface language.',
    onChange: noop,
  } satisfies ComponentProps<typeof Dropdown>,
  TextField: {
    label: 'Link title',
    value: 'Insta',
    placeholder: 'e.g. Instagram',
    maxLength: 15,
    showCounter: true,
    onChangeText: noop,
    onClear: noop,
  } satisfies ComponentProps<typeof TextField>,
  SearchField: {
    value: 'Casa',
    placeholder: 'Search',
    onChangeText: noop,
    onBack: noop,
  } satisfies ComponentProps<typeof SearchField>,
  Dialog: {
    visible: true,
    icon: 'warning',
    title: 'Leave without saving?',
    body: "You'll lose any unsaved changes.",
    buttons: [
      { label: 'LEAVE', tone: 'secondary', onPress: noop },
      { label: 'STAY', onPress: noop },
    ],
    onRequestClose: noop,
  } satisfies ComponentProps<typeof Dialog>,
  Toast: {
    message: 'Version copied to clipboard',
    icon: 'check',
  } satisfies ComponentProps<typeof ToastBubble>,
  BottomSheet: {
    snap: 'half',
    onSnapChange: noop,
    containerHeight: 640,
  } satisfies ComponentProps<typeof BottomSheet>,
  Skeleton: { width: 147, height: 77, radius: 8 } satisfies ComponentProps<
    typeof Skeleton
  >,
  EmptyState: {
    icon: 'wifiOff',
    title: 'Connection lost',
    body: 'Check your internet and try again.',
    actionLabel: 'RETRY',
    onAction: noop,
  } satisfies ComponentProps<typeof EmptyState>,
  StatusPill: { kind: 'live', label: 'LIVE' } satisfies ComponentProps<
    typeof StatusPill
  >,
  Chip: {
    label: 'Instagram',
    icon: 'link',
    onPress: noop,
    onRemove: noop,
  } satisfies ComponentProps<typeof Chip>,
  ProgressBar: {
    value: 0.5,
    label: 'Current Cache Size',
  } satisfies ComponentProps<typeof ProgressBar>,
  OutlinedButton: {
    label: 'CLEAN CACHE',
    onPress: noop,
  } satisfies ComponentProps<typeof OutlinedButton>,
  CollapsibleSection: {
    title: 'LINKS',
    open: false,
    onToggle: noop,
  } satisfies ComponentProps<typeof CollapsibleSection>,
  LinkRow: {
    label: 'Privacy Policy',
    external: true,
    onPress: noop,
  } satisfies ComponentProps<typeof LinkRow>,
  WarningBanner: {
    message: 'Device no longer supported',
  } satisfies ComponentProps<typeof WarningBanner>,
  PageDots: { count: 4, index: 1 } satisfies ComponentProps<typeof PageDots>,
  SnapCarousel: {
    data: ['a', 'b', 'c'],
    itemWidth: 294,
    gap: 12,
    keyOf: (item: string) => item,
    renderItem: () => null,
  } satisfies ComponentProps<typeof SnapCarousel<string>>,
  HudCircleButton: {
    icon: 'chat',
    label: 'Chat',
    badge: 3,
    onPress: noop,
  } satisfies ComponentProps<typeof HudCircleButton>,
  ReadOnlyField: {
    label: 'Email',
    value: 'name@email.com',
  } satisfies ComponentProps<typeof ReadOnlyField>,
  ActionSquareButton: {
    icon: 'share',
    label: 'Share',
    onPress: noop,
  } satisfies ComponentProps<typeof ActionSquareButton>,
};
