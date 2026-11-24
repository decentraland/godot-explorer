// Design System Mobile components added on 2026-11-17: Button/SeeMore, Button/ProfileLink,
// Profile/Attribute, Icon/Profile, Menu/NavItem, Settings/Row, Input/Username and Modal/Special.
import { useState } from 'react';
import { Image, ScrollView, StyleSheet, View } from 'react-native';
import {
  AppText,
  BrandBackground,
  Button,
  ButtonProfileLink,
  ButtonSeeMore,
  IconProfile,
  InputUsername,
  MenuNavItem,
  ModalSpecial,
  ProfileAttribute,
  ProfileIconType,
  ScrollFade,
  SettingsRow,
  Slider,
  radius,
  color,
  space,
  Switch,
  u,
} from '@dcl/ui-native';
import { registerPreview } from './registry';

const media = require('./assets/modal-special-media.jpg');
const noop = () => {};
const PROFILE_TYPES: ProfileIconType[] = [
  'Gender',
  'Country',
  'Birthday',
  'Pronouns',
  'Relationship',
  'SexOrientation',
  'Language',
  'Profession',
  'Hobbies',
  'RealName',
  'EmploymentStatus',
];

function Additions() {
  const [on, setOn] = useState(true);
  const [volume, setVolume] = useState(100);
  return (
    <ScrollView contentContainerStyle={styles.content}>
      <View style={styles.row}>
        <ButtonSeeMore onPress={noop} />
        <ButtonSeeMore expanded onPress={noop} />
        <ButtonSeeMore disabled onPress={noop} />
      </View>
      <View style={styles.row}>
        <ButtonProfileLink label="Link" onPress={noop} />
        <ButtonProfileLink label="Link" onPress={noop} onRemove={noop} />
      </View>
      <View style={styles.row}>
        {PROFILE_TYPES.map(type => (
          <IconProfile key={type} type={type} />
        ))}
      </View>
      <ScrollFade edge="Both">
        <ScrollView horizontal contentContainerStyle={styles.fadeRow}>
          {PROFILE_TYPES.map(type => (
            <View key={type} style={styles.tile} />
          ))}
        </ScrollView>
      </ScrollFade>
      <View style={styles.row}>
        <ProfileAttribute icon="Country" title="Country" value="Argentina" />
        <ProfileAttribute icon="Language" title="Language" value="Spanish" />
      </View>
      <View>
        <MenuNavItem label="Label" onPress={noop} />
        <MenuNavItem label="Label" selected onPress={noop} />
        <MenuNavItem label="Label" size="Small" onPress={noop} />
        <MenuNavItem label="Label" size="Small" selected onPress={noop} />
      </View>
      <View>
        <SettingsRow
          label="Label"
          control={<Switch label="Label" value={on} onChange={setOn} />}
        />
        <SettingsRow
          type="SwitchDescription"
          label="Label"
          description="Adjusts graphics automatically for optimal performance."
          control={<Switch label="Label" value={on} onChange={setOn} />}
        />
        <SettingsRow
          type="Slider"
          label="Label"
          value={`${volume}%`}
          control={
            <Slider
              label="Label"
              value={volume}
              onChange={setVolume}
              showLabel={false}
            />
          }
        />
      </View>
      <InputUsername
        label="Username"
        uniqueName
        tab="Unique"
        onTabChange={noop}
        uniqueTabLabel="Unique name"
        nameTabLabel="Non-unique username"
        names={['Name']}
        picked="Name"
        onPick={noop}
        value=""
        onChangeText={noop}
      />
      <InputUsername
        label="Username"
        uniqueName={false}
        tab="Name"
        onTabChange={noop}
        uniqueTabLabel="Unique name"
        nameTabLabel="Non-unique username"
        names={[]}
        picked={null}
        onPick={noop}
        value="username"
        onChangeText={noop}
        suffix="#1234"
        maxLength={15}
      />
    </ScrollView>
  );
}

registerPreview('extra/ds-figma-additions', {
  title: 'DS: components added from Figma',
  orientation: 'portrait',
  render: () => (
    <View style={styles.page}>
      <BrandBackground />
      <Additions />
    </View>
  ),
});

registerPreview('extra/ds-modal-special', {
  title: 'DS: Modal/Special',
  orientation: 'portrait',
  notes: 'state=custom: Type=Custom with a caller-built layout',
  render: state => (
    <View style={styles.page}>
      <BrandBackground />
      {state === 'custom' ? (
        <ModalSpecial type="custom" visible onRequestClose={noop}>
          <View style={styles.custom}>
            <AppText variant="Modal/Special Title" style={styles.customTitle}>
              Season Pass is live
            </AppText>
            <Button label="See rewards" onPress={noop} fullWidth />
          </View>
        </ModalSpecial>
      ) : (
        <ModalSpecial
          visible
          media={<Image source={media} style={styles.media} />}
          title="Unlock the full experience"
          body="Add email to unlock Decentraland across devices and claim an"
          highlight="exclusive Wearable."
          primaryLabel="Add email"
          onPrimary={noop}
          secondaryLabel="Maybe later"
          onSecondary={noop}
          onRequestClose={noop}
        />
      )}
    </View>
  ),
});

const styles = StyleSheet.create({
  page: { flex: 1 },
  content: {
    paddingHorizontal: space['2xl'],
    paddingVertical: space.xl,
    gap: space.xl,
  },
  row: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: space.lg,
  },
  fadeRow: { gap: space.xs },
  tile: {
    width: u(40),
    height: u(40),
    borderRadius: radius.lg,
    backgroundColor: color.action.selected,
  },
  media: { width: u(344), height: u(274), marginTop: -u(9.5) },
  custom: {
    flex: 1,
    justifyContent: 'flex-end',
    gap: space['54'],
    paddingHorizontal: space['50'],
    paddingBottom: space['40'],
  },
  customTitle: { color: color.text.primary, textAlign: 'center' },
});
