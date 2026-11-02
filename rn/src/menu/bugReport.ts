import type { I18nKey } from '../i18n';

export const MAX_DESCRIPTION = 300;
export const MAX_SCREENSHOTS = 3;

export const CATEGORIES: { value: string; label: I18nKey }[] = [
  { value: 'performance', label: 'BUG_REPORT_CATEGORY_PERFORMANCE' },
  { value: 'crash', label: 'BUG_REPORT_CATEGORY_CRASH' },
  { value: 'chat', label: 'BUG_REPORT_CATEGORY_CHAT' },
  { value: 'streaming', label: 'BUG_REPORT_CATEGORY_STREAMING' },
  { value: 'friends', label: 'BUG_REPORT_CATEGORY_FRIENDS' },
  { value: 'wearables', label: 'BUG_REPORT_CATEGORY_WEARABLES' },
  { value: 'profile', label: 'BUG_REPORT_CATEGORY_PROFILE' },
  { value: 'other', label: 'BUG_REPORT_CATEGORY_OTHER' },
];

export type BugForm = {
  category: string | null;
  description: string;
  screenshots: string[];
};

export const emptyForm: BugForm = {
  category: null,
  description: '',
  screenshots: [],
};

/** Both the issue type and a description are required. */
export const canSubmit = (form: BugForm): boolean =>
  form.category !== null && form.description.trim().length > 0;

export const addScreenshot = (form: BugForm, uri: string): BugForm =>
  form.screenshots.length >= MAX_SCREENSHOTS
    ? form
    : { ...form, screenshots: [...form.screenshots, uri] };

export const removeScreenshot = (form: BugForm, index: number): BugForm => ({
  ...form,
  screenshots: form.screenshots.filter((_, i) => i !== index),
});

export const withDescription = (form: BugForm, text: string): BugForm => ({
  ...form,
  description: text.slice(0, MAX_DESCRIPTION),
});
