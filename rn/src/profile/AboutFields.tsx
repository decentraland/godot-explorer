import { StyleSheet, View } from 'react-native';
import { AppText, colors, fonts, Icon, IconName } from '../ds';
import type { I18nKey } from '../i18n';
import { t } from '../i18n';
import {
  GENDERS,
  optionLabel,
  ORIENTATIONS,
  PRONOUNS,
  RELATIONSHIPS,
} from './options';
import type { AboutKey } from './logic';
import { u } from '../ds/scale';
import type { Profile } from './types';

const META: Record<AboutKey, { label: I18nKey; icon: IconName }> = {
  country: { label: 'PROFILE_COUNTRY', icon: 'globe' },
  language: { label: 'PROFILE_LANGUAGE', icon: 'chat' },
  pronouns: { label: 'PROFILE_PRONOUNS', icon: 'people' },
  gender: { label: 'PROFILE_GENDER', icon: 'user' },
  relationshipStatus: { label: 'PROFILE_RELATIONSHIP_STATUS', icon: 'heart' },
  sexualOrientation: { label: 'PROFILE_SEXUAL_ORIENTATION', icon: 'heart' },
  employmentStatus: { label: 'PROFILE_EMPLOYMENT_STATUS', icon: 'clock' },
  profession: { label: 'PROFILE_PROFESSION', icon: 'flag' },
  realName: { label: 'PROFILE_REAL_NAME', icon: 'user' },
  hobbies: { label: 'PROFILE_HOBBY', icon: 'star' },
};

const display = (profile: Profile, key: AboutKey): string => {
  switch (key) {
    case 'pronouns':
      return optionLabel(PRONOUNS, profile.pronouns, t);
    case 'gender':
      return optionLabel(GENDERS, profile.gender, t);
    case 'sexualOrientation':
      return optionLabel(ORIENTATIONS, profile.sexualOrientation, t);
    case 'relationshipStatus':
      return optionLabel(RELATIONSHIPS, profile.relationshipStatus, t);
    default:
      return profile[key];
  }
};

/** Icon + caps label over the value, laid out in `columns` equal columns. */
export function AboutFields({
  profile,
  keys,
  columns,
}: {
  profile: Profile;
  keys: AboutKey[];
  columns: 2 | 3;
}) {
  const rows: AboutKey[][] = [];
  for (let i = 0; i < keys.length; i += columns) {
    rows.push(keys.slice(i, i + columns));
  }
  return (
    <View>
      {rows.map(row => (
        <View key={row[0]} style={styles.row}>
          {Array.from({ length: columns }, (_, i) => {
            const key = row[i];
            return (
              <View key={key ?? `empty${i}`} style={styles.cell}>
                {key && (
                  <>
                    <View style={styles.label}>
                      <Icon
                        name={META[key].icon}
                        size={10}
                        color={colors.cardLight}
                        strokeWidth={2}
                      />
                      <AppText variant="label" style={styles.labelText}>
                        {t(META[key].label)}
                      </AppText>
                    </View>
                    <AppText variant="body" style={styles.value}>
                      {display(profile, key)}
                    </AppText>
                  </>
                )}
              </View>
            );
          })}
        </View>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', marginBottom: u(8) },
  cell: { flex: 1 },
  label: { flexDirection: 'row', alignItems: 'center', gap: u(4) },
  labelText: {
    color: colors.cardLight,
    fontFamily: fonts.medium,
    fontSize: u(10),
  },
  value: { color: colors.snow, fontSize: u(12), marginTop: u(2) },
});
