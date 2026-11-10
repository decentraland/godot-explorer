import { parse, SvgAst, SvgXml } from 'react-native-svg';
import {
  badgePaths,
  basePaths,
  buttonPaths,
  creditsSvg,
  discordSvg,
  Glyph,
  GlyphFamily,
  googleSvg,
  logoPaths,
  menuBanMemberSvg,
  menuClearChatSvg,
  menuDemoteSpeakerSvg,
  menuJumpSvg,
  menuLeaveSvg,
  menuMakeSpeakerSvg,
  menuPaths,
  menuRemoveMemberSvg,
  menuRevokeOpSvg,
  modalAlertSvg,
  modalBannedSvg,
  modalBlockSvg,
  modalBugSvg,
  modalConnectionSvg,
  modalPasswordSvg,
  modalReportSvg,
  modalSuccessSvg,
  nameLogoSvg,
  namePaths,
  nameVerifiedSvg,
  navBarPaths,
  navProfileActiveSvg,
  navProfileSvg,
  settingsPaths,
  strokePaths,
  walletConnectSvg,
  wearablesSvg,
} from './iconSvgs';
import { u } from './scale';

const sources = {
  credits: creditsSvg,
  discord: discordSvg,
  google: googleSvg,
  menuBanMember: menuBanMemberSvg,
  menuClearChat: menuClearChatSvg,
  menuDemoteSpeaker: menuDemoteSpeakerSvg,
  menuJump: menuJumpSvg,
  menuLeave: menuLeaveSvg,
  menuMakeSpeaker: menuMakeSpeakerSvg,
  menuRemoveMember: menuRemoveMemberSvg,
  menuRevokeOp: menuRevokeOpSvg,
  modalAlert: modalAlertSvg,
  modalBanned: modalBannedSvg,
  modalBlock: modalBlockSvg,
  modalBug: modalBugSvg,
  modalConnection: modalConnectionSvg,
  modalPassword: modalPasswordSvg,
  modalReport: modalReportSvg,
  modalSuccess: modalSuccessSvg,
  nameLogo: nameLogoSvg,
  nameVerified: nameVerifiedSvg,
  navProfile: navProfileSvg,
  navProfileActive: navProfileActiveSvg,
  walletConnect: walletConnectSvg,
  wearables: wearablesSvg,
};

/** Parsed once: re-parsing the exported SVGs on every mount occasionally rendered them empty on device. */
const asts = Object.fromEntries(
  Object.entries(sources).map(([name, xml]) => [name, parse(xml)]),
);

type Mono = { viewBox: string; inner: string; fill?: string };

function prefixed<P extends string, K extends string>(
  prefix: P,
  family: { paths: Record<K, Glyph> },
): Record<`${P}${Capitalize<K>}`, Glyph> {
  return Object.fromEntries(
    Object.entries(family.paths).map(([k, v]) => [
      `${prefix}${k[0].toUpperCase()}${k.slice(1)}`,
      v,
    ]),
  ) as Record<`${P}${Capitalize<K>}`, Glyph>;
}

const monoFamilies: { family: GlyphFamily; paths: Record<string, Glyph> }[] = [
  { family: basePaths, paths: basePaths.paths },
  { family: buttonPaths, paths: buttonPaths.paths },
  { family: menuPaths, paths: prefixed('menu', menuPaths) },
  { family: navBarPaths, paths: prefixed('nav', navBarPaths) },
  { family: settingsPaths, paths: prefixed('settings', settingsPaths) },
  { family: logoPaths, paths: logoPaths.paths },
  { family: badgePaths, paths: badgePaths.paths },
  { family: namePaths, paths: namePaths.paths },
];

const monos: Record<string, Mono> = {};
for (const { family, paths } of monoFamilies) {
  for (const [name, glyph] of Object.entries(paths)) {
    const g = typeof glyph === 'string' ? { d: glyph } : glyph;
    monos[name] = {
      viewBox: g.viewBox ?? `0 0 ${family.viewBox} ${family.viewBox}`,
      inner:
        'markup' in g
          ? g.markup
          : `<path d="${g.d}"${
              g.fillRule ? ` fill-rule="${g.fillRule}"` : ''
            } fill="currentColor"/>`,
      fill: g.fill,
    };
  }
}

const strokes: Record<string, string> = strokePaths;

/** Names from before the design-system sweep, kept so call sites and tests keep working. */
const aliases = {
  backpack: 'navBackpackActive',
  backpackOutline: 'navBackpack',
  bell: 'notification',
  bug: 'menuBug',
  chevronLeft: 'back',
  compass: 'navExplore',
  compassActive: 'navExploreActive',
  compassFilled: 'navExploreActive',
  compassOutline: 'navExplore',
  dancer: 'emote',
  emotes: 'emote',
  flag: 'menuReport',
  friends: 'menuFriends',
  globe: 'world',
  heart: 'fav',
  info: 'menuInfo',
  jump: 'jumpIn',
  kebab: 'menuDot',
  link: 'menuLink',
  logout: 'leave',
  mail: 'email',
  pencil: 'menuEditProfile',
  people: 'menuCommunity',
  peopleHeart: 'navSocial',
  personPlus: 'menuAddFriend',
  pin: 'location',
  profile: 'menuPlayer',
  profileFill: 'navProfile',
  profileRing: 'navProfileRing',
  seal: 'nameVerified',
  settings: 'navSettings',
  settingsActive: 'navSettingsActive',
  sliders: 'filter',
  trash: 'menuDelete',
  user: 'players',
  warning: 'alert',
  wifiOff: 'noConnection',
} as const;

type MonoName =
  | keyof typeof basePaths.paths
  | keyof typeof buttonPaths.paths
  | `menu${Capitalize<keyof typeof menuPaths.paths>}`
  | `nav${Capitalize<keyof typeof navBarPaths.paths>}`
  | `settings${Capitalize<keyof typeof settingsPaths.paths>}`
  | keyof typeof logoPaths.paths
  | keyof typeof badgePaths.paths
  | keyof typeof namePaths.paths;

export type IconName =
  | keyof typeof sources
  | MonoName
  | keyof typeof strokePaths
  | keyof typeof aliases;

export const iconNames = [
  ...Object.keys(monos),
  ...Object.keys(sources),
  ...Object.keys(strokes),
  ...Object.keys(aliases),
] as IconName[];

export function Icon({
  name: requested,
  size: designSize = 24,
  color,
  strokeWidth = 2.4,
  fill,
}: {
  name: IconName;
  size?: number;
  /** Defaults to snow, or to the glyph's own colour for the ones Figma paints (nav actives, badges, logos). */
  color?: string;
  strokeWidth?: number;
  /** Fills the stroke icon's shapes (even-odd, so nested outlines cut holes). */
  fill?: string;
}) {
  const size = u(designSize);
  const name: string =
    requested in aliases
      ? aliases[requested as keyof typeof aliases]
      : requested;
  if (name in monos) {
    const mono = monos[name];
    const paint = color ?? mono.fill ?? '#FCFCFC';
    const xml = `<svg viewBox="${mono.viewBox}" color="${paint}">${mono.inner}</svg>`;
    return <SvgXml xml={xml} width={size} height={size} />;
  }
  if (name in strokes) {
    const xml = `<svg viewBox="0 0 24 24" fill="none"><path d="${
      strokes[name]
    }" fill="${fill ?? 'none'}" fill-rule="evenodd" stroke="${
      color ?? '#FCFCFC'
    }" stroke-width="${strokeWidth}" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
    return <SvgXml xml={xml} width={size} height={size} />;
  }
  return <SvgAst ast={asts[name]} override={{ width: size, height: size }} />;
}
