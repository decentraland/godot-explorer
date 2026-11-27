// Design System Mobile components added on 2026-11-17: Button/SeeMore, Button/ProfileLink,
// Profile/Attribute, Icon/Profile, Menu/NavItem, Settings/Row, Input/Username and Modal/Special;
// Loader/Logo, Image Not Found, Background/Avatar, Tooltip, Alert and Notification on
// 2026-11-25; Badge-DCL, Badge-Small, Badge-ProfileRow and Bar on 2026-11-27.
import { useState } from 'react';
import { Image, ScrollView, StyleSheet, View } from 'react-native';
import {
  Alert,
  AppText,
  AvatarBackground,
  BadgeBar,
  BadgeDCL,
  BadgeProfileRow,
  BadgeSmall,
  BrandBackground,
  Button,
  ButtonProfileLink,
  ButtonSeeMore,
  IconProfile,
  ImageNotFound,
  InputUsername,
  LogoLoader,
  MenuNavItem,
  ModalSpecial,
  Notification,
  ProfileAttribute,
  ProfileIconType,
  ScrollFade,
  SettingsRow,
  Slider,
  radius,
  color,
  space,
  Switch,
  Tooltip,
  u,
} from '@dcl/ui-native';
import { registerPreview } from './registry';

const media = require('./assets/modal-special-media.jpg');
const badgeArt = require('./assets/badge-explorer-bronze.png');
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
      <View style={styles.row}>
        <LogoLoader accessibilityLabel="Loading" />
        <View style={styles.notFound}>
          <ImageNotFound />
        </View>
        <View style={styles.spotlight}>
          <AvatarBackground />
        </View>
      </View>
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

registerPreview('extra/ds-tooltip', {
  title: 'DS: Tooltip',
  orientation: 'portrait',
  render: () => (
    <ScrollView
      style={styles.tooltipPage}
      contentContainerStyle={styles.tooltips}
    >
      <Tooltip text="Tap Chat to talk with others" />
      <Tooltip text="Hold the jump button longer to jump higher, and tap it again in the air to double jump." />
      <Tooltip arrow="left" text="Chat" style={styles.tooltipLeft} />
      <Tooltip arrow="top" text="Menu" />
      <Tooltip arrow="right" text="Emotes" />
      <Tooltip arrow="bottom" text="Map" />
      <Tooltip text="Open the backpack to change what you wear" />
    </ScrollView>
  ),
});

registerPreview('extra/ds-alert', {
  title: 'DS: Alert and Notification',
  orientation: 'portrait',
  render: () => (
    <View style={styles.page}>
      <BrandBackground />
      <ScrollView contentContainerStyle={styles.alerts}>
        <Notification
          title="Unlock the full experience"
          body="Add email to play across mobile and desktop, and claim an exclusive Wearable."
          actionLabel="Add email"
          onAction={noop}
        />
        <Alert lead="Reward claim failed." message="Try again." />
        <Alert
          lead="Reward claim failed."
          message="Check your connection and try again."
        />
        <Alert background="translucent" message="Device no longer supported" />
      </ScrollView>
    </View>
  ),
});

registerPreview('extra/ds-badges', {
  title: 'DS: Badges',
  orientation: 'landscape',
  render: () => (
    <View style={styles.page}>
      <BrandBackground />
      <ScrollView contentContainerStyle={styles.badges}>
        <View style={styles.row}>
          <BadgeProfileRow image={badgeArt} selected />
          <BadgeProfileRow image={badgeArt} />
          <BadgeProfileRow />
          <BadgeSmall image={badgeArt} />
          <BadgeSmall image={badgeArt} state="selected" />
          <BadgeSmall image={badgeArt} state="disabled" />
        </View>
        <View style={styles.row}>
          <BadgeDCL image={badgeArt} name="Traveler bronze" progress={0} />
          <BadgeDCL image={badgeArt} name="Traveler bronze" progress={0.4} />
          <BadgeDCL
            image={badgeArt}
            name="Traveler bronze"
            date="Jan. 2024"
            state="selected"
          />
          <BadgeDCL
            image={badgeArt}
            name="Traveler bronze"
            progress={0}
            state="disabled"
          />
          <BadgeDCL
            image={badgeArt}
            name="Traveler bronze"
            date="Jan. 2024"
            state="disabled"
          />
        </View>
        <View style={styles.row}>
          <BadgeBar value={0} />
          <BadgeBar value={0.4} />
          <BadgeBar value={1} />
        </View>
      </ScrollView>
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
  badges: { padding: space.xl, gap: space.xl },
  notFound: { width: u(147), height: u(78) },
  spotlight: { width: u(160), height: u(172) },
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
  tooltipPage: { flex: 1, backgroundColor: color.background.elevated },
  tooltips: { padding: space['2xl'], gap: space['2xl'] },
  tooltipLeft: { marginLeft: space.lg },
  alerts: { padding: space['2xl'], gap: space.xl },
});
