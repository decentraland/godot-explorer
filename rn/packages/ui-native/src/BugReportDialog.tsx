import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Dialog } from './Dialog';
import { Dropdown, DropdownOption } from './Dropdown';
import { ScreenshotCard } from './ScreenshotCard';
import { TextField } from './TextField';
import { color, space } from './tokens';

export const BUG_DESCRIPTION_MAX = 300;

/** Bug report form: the Bug dialog with an issue type, a description (300 characters) and an
 * optional screenshot. Submit stays disabled until both required fields are filled. The caller
 * passes every translated string.
 * Figma Modal/BugReport: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1097-394
 */
export function BugReportDialog({
  visible,
  labels,
  issueTypes,
  issueType,
  onIssueTypeChange,
  description,
  onDescriptionChange,
  screenshotUri,
  onAddScreenshot,
  onRemoveScreenshot,
  onSubmit,
  onRequestClose,
}: {
  visible: boolean;
  labels: {
    title: string;
    issueType: string;
    select: string;
    description: string;
    descriptionPlaceholder: string;
    screenshot: string;
    addScreenshot: string;
    cancel: string;
    submit: string;
  };
  issueTypes: DropdownOption[];
  issueType: string | null;
  onIssueTypeChange: (value: string) => void;
  description: string;
  onDescriptionChange: (text: string) => void;
  screenshotUri?: string;
  onAddScreenshot: () => void;
  onRemoveScreenshot: () => void;
  onSubmit: () => void;
  onRequestClose: () => void;
}) {
  const ready = issueType !== null && description.trim().length > 0;
  return (
    <Dialog
      visible={visible}
      icon="bug"
      title={labels.title}
      onRequestClose={onRequestClose}
      buttons={[
        { label: labels.cancel, tone: 'secondary', onPress: onRequestClose },
        { label: labels.submit, onPress: onSubmit, disabled: !ready },
      ]}
    >
      <View style={styles.fields}>
        <Dropdown
          label={`${labels.issueType}*`}
          placeholder={labels.select}
          value={issueType}
          options={issueTypes}
          onChange={onIssueTypeChange}
        />
        <TextField
          label={`${labels.description}*`}
          placeholder={labels.descriptionPlaceholder}
          value={description}
          onChangeText={onDescriptionChange}
          multiline
          maxLength={BUG_DESCRIPTION_MAX}
          showCounter
        />
        <View style={styles.shots}>
          <AppText variant="H5" style={styles.label}>
            {labels.screenshot}
          </AppText>
          <View style={styles.row}>
            {screenshotUri !== undefined && (
              <ScreenshotCard
                uri={screenshotUri}
                accessibilityLabel={labels.screenshot}
                onPress={onAddScreenshot}
                onRemove={onRemoveScreenshot}
              />
            )}
            <ScreenshotCard
              accessibilityLabel={labels.addScreenshot}
              onPress={onAddScreenshot}
            />
          </View>
        </View>
      </View>
    </Dialog>
  );
}

const styles = StyleSheet.create({
  fields: { gap: space.xl, alignSelf: 'stretch' },
  shots: { gap: space['10'] },
  label: { color: color.text.primary },
  row: { flexDirection: 'row', gap: space.xl },
});
