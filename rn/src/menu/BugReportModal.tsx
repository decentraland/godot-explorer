import { useEffect, useState } from 'react';
import {
  Image,
  Pressable,
  ScrollView,
  StyleSheet,
  TextInput,
  View,
} from 'react-native';
import { AppText, colors, Dropdown, Icon, LinearFill, u } from '../ds';
import { ArtFill } from './ArtFill';
import { counterLabel } from '../ds/TextField';
import { t } from '../i18n';
import {
  addScreenshot,
  BugForm,
  canSubmit,
  CATEGORIES,
  emptyForm,
  MAX_DESCRIPTION,
  MAX_SCREENSHOTS,
  removeScreenshot,
  withDescription,
} from './bugReport';

const SLOT = u(85);
const FIELD = '#350854';

export type BugReportModalProps = {
  /** Resolves a world capture; called once on open for the first slot and on "+". */
  capture: () => Promise<string>;
  onSubmit: (form: BugForm) => Promise<void>;
  onClose: () => void;
  initial?: BugForm;
  /** Gallery state: the success card. */
  sent?: boolean;
};

function Label({ text, required }: { text: string; required?: boolean }) {
  return (
    <AppText variant="body" style={styles.label}>
      {text}
      {required && (
        <AppText variant="body" style={styles.star}>
          *
        </AppText>
      )}
    </AppText>
  );
}

export function BugReportModal({
  capture,
  onSubmit,
  onClose,
  initial = emptyForm,
  sent: initialSent = false,
}: BugReportModalProps) {
  const [form, setForm] = useState(initial);
  const [sent, setSent] = useState(initialSent);
  const [failed, setFailed] = useState(false);
  const [busy, setBusy] = useState(false);

  const attach = () =>
    capture()
      .then(uri => setForm(f => addScreenshot(f, uri)))
      .catch(() => undefined);

  useEffect(() => {
    if (initial.screenshots.length === 0) {
      attach();
    }
    // Only the first open captures on its own.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const submit = () => {
    setBusy(true);
    setFailed(false);
    onSubmit(form)
      .then(() => setSent(true))
      .catch(() => setFailed(true))
      .finally(() => setBusy(false));
  };
  const valid = canSubmit(form) && !busy;

  return (
    <View style={styles.scrim}>
      <View style={styles.card} accessibilityViewIsModal>
        <View style={styles.bg}>
          <LinearFill
            stops={[
              { offset: 0, color: '#4F1580' },
              { offset: 1, color: '#521A82' },
            ]}
          />
        </View>
        <View style={styles.bug} pointerEvents="none">
          <Icon name="bug" size={34} color={colors.snow} strokeWidth={5} />
          <View style={styles.bugTop}>
            <Icon
              name="bug"
              size={34}
              color="#7A2B00"
              strokeWidth={1.4}
              fill={colors.orange}
            />
          </View>
        </View>
        {sent ? (
          <View style={styles.sent}>
            <AppText variant="heading" style={styles.title}>
              {t('MENU_BUG_SENT_TITLE')}
            </AppText>
            <AppText variant="body" style={styles.sentBody}>
              {t('MENU_BUG_SENT_BODY')}
            </AppText>
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={t('MENU_CLOSE')}
              onPress={onClose}
              style={[styles.button, styles.cancel]}
            >
              <AppText variant="button" style={styles.buttonText}>
                {t('MENU_CLOSE').toUpperCase()}
              </AppText>
            </Pressable>
          </View>
        ) : (
          <ScrollView
            keyboardShouldPersistTaps="handled"
            contentContainerStyle={styles.body}
          >
            <AppText variant="heading" style={styles.title}>
              {t('BUG_REPORT_TITLE')}
            </AppText>
            <Label text={t('BUG_REPORT_ISSUE_TYPE')} required />
            <View style={styles.dropdown}>
              <Dropdown
                label=""
                value={form.category}
                placeholder={t('MENU_BUG_SELECT')}
                options={CATEGORIES.map(c => ({
                  value: c.value,
                  label: t(c.label),
                }))}
                onChange={category => setForm(f => ({ ...f, category }))}
              />
            </View>
            <Label text={t('BUG_REPORT_DESCRIPTION')} required />
            <TextInput
              multiline
              value={form.description}
              onChangeText={text => setForm(f => withDescription(f, text))}
              placeholder={t('BUG_REPORT_DESCRIPTION_PLACEHOLDER')}
              placeholderTextColor="rgba(255,255,255,0.45)"
              maxLength={MAX_DESCRIPTION}
              textAlignVertical="top"
              style={styles.textarea}
            />
            <AppText variant="caption" style={styles.counter}>
              {counterLabel(form.description.length, MAX_DESCRIPTION)}
            </AppText>
            <Label text={t('BUG_REPORT_SCREENSHOT')} />
            <View style={styles.shots}>
              {form.screenshots.map((uri, i) => (
                <View key={`${uri}${i}`} style={styles.slot}>
                  {uri && !uri.startsWith('mock://') ? (
                    <Image
                      source={{ uri }}
                      style={styles.fill}
                      resizeMode="cover"
                    />
                  ) : (
                    <ArtFill seed="screenshot" />
                  )}
                  <Pressable
                    accessibilityRole="button"
                    accessibilityLabel={t('MENU_BUG_REMOVE_SCREENSHOT')}
                    hitSlop={u(6)}
                    onPress={() => setForm(f => removeScreenshot(f, i))}
                    style={styles.remove}
                  >
                    <Icon
                      name="close"
                      size={8}
                      color={colors.snow}
                      strokeWidth={3}
                    />
                  </Pressable>
                </View>
              ))}
              {form.screenshots.length < MAX_SCREENSHOTS && (
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel={t('MENU_BUG_ADD_SCREENSHOT')}
                  onPress={attach}
                  style={[styles.slot, styles.add]}
                >
                  <Icon
                    name="plus"
                    size={20}
                    color={colors.lavender70}
                    strokeWidth={2}
                  />
                </Pressable>
              )}
            </View>
            {failed && (
              <AppText variant="caption" style={styles.error}>
                {t('MENU_BUG_FAILED')}
              </AppText>
            )}
            <View style={styles.buttons}>
              <Pressable
                accessibilityRole="button"
                accessibilityLabel={t('COMMON_CANCEL')}
                onPress={onClose}
                style={[styles.button, styles.cancel]}
              >
                <AppText variant="button" style={styles.buttonText}>
                  {t('COMMON_CANCEL')}
                </AppText>
              </Pressable>
              <Pressable
                accessibilityRole="button"
                accessibilityLabel={t('COMMON_SUBMIT')}
                accessibilityState={{ disabled: !valid }}
                disabled={!valid}
                onPress={submit}
                style={[styles.button, styles.submit]}
              >
                <AppText
                  variant="button"
                  style={[styles.buttonText, !valid && styles.dimText]}
                >
                  {t('COMMON_SUBMIT')}
                </AppText>
              </Pressable>
            </View>
          </ScrollView>
        )}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  scrim: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: 'rgba(22,21,24,0.62)',
  },
  card: {
    width: u(312),
    maxHeight: '78%',
    borderRadius: u(10),
    overflow: 'visible',
  },
  bg: {
    ...StyleSheet.absoluteFill,
    borderRadius: u(10),
    overflow: 'hidden',
  },
  bug: { position: 'absolute', top: -u(17), alignSelf: 'center', zIndex: 2 },
  bugTop: { position: 'absolute' },
  body: { padding: u(24), paddingTop: u(40) },
  title: { fontSize: u(20), textAlign: 'center', marginBottom: u(14) },
  label: { fontSize: u(15), fontFamily: 'Inter-Medium', color: colors.snow },
  star: { color: colors.ruby },
  dropdown: { marginTop: u(6), marginBottom: u(12) },
  textarea: {
    height: u(108),
    marginTop: u(8),
    borderRadius: u(4),
    padding: u(8),
    fontSize: u(14),
    fontFamily: 'Inter-Regular',
    color: colors.snow,
    backgroundColor: FIELD,
  },
  counter: { marginTop: u(4), marginBottom: u(12) },
  shots: { flexDirection: 'row', gap: u(16), marginTop: u(8) },
  slot: {
    width: SLOT,
    height: SLOT,
    borderRadius: u(6),
    overflow: 'hidden',
    backgroundColor: FIELD,
  },
  fill: { width: '100%', height: '100%' },
  add: { alignItems: 'center', justifyContent: 'center' },
  remove: {
    position: 'absolute',
    top: u(2),
    right: u(2),
    width: u(14),
    height: u(14),
    borderRadius: u(7),
    backgroundColor: '#000',
    alignItems: 'center',
    justifyContent: 'center',
  },
  error: { color: colors.ruby, marginTop: u(10) },
  buttons: { flexDirection: 'row', gap: u(16), marginTop: u(22) },
  button: {
    flex: 1,
    height: u(37),
    borderRadius: u(6),
    alignItems: 'center',
    justifyContent: 'center',
  },
  cancel: { backgroundColor: FIELD },
  submit: { backgroundColor: '#A52068' },
  buttonText: { fontFamily: 'Inter-Bold', fontSize: u(14) },
  dimText: { opacity: 0.5 },
  sent: { padding: u(24), paddingTop: u(40), alignItems: 'center', gap: u(12) },
  sentBody: { textAlign: 'center', fontSize: u(14), color: colors.white70 },
});
