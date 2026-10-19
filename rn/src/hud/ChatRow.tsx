import { StyleSheet, Text, View } from 'react-native';
import { colors, fonts, Icon, u } from '../ds';
import { t } from '../i18n';
import { parseBbcode } from './bbcode';
import {
  rowKind,
  Segment,
  splitName,
  tokenize,
  TokenizeContext,
  truncateReduced,
} from './chatModel';
import type { ChatLine, ChatProfile } from './types';

const LINK = '#66B3FF';
const MENTION = '#FFD700';
const MY_MENTION = '#FC03EC';
const SYSTEM_GREEN = '#00ff00';

const background = {
  other: 'rgba(0,0,0,0.4)',
  own: 'rgba(0,0,0,0.7)',
  system: 'rgba(0,0,0,0.4)',
} as const;

function Body({
  segments,
  onLink,
}: {
  segments: Segment[];
  onLink?: (segment: Segment) => void;
}) {
  return (
    <>
      {segments.map((segment, i) => {
        const tappable = segment.kind !== 'text' && !!onLink;
        const color =
          segment.kind === 'mention'
            ? segment.mine
              ? MY_MENTION
              : MENTION
            : segment.kind === 'text'
            ? undefined
            : LINK;
        return (
          <Text
            key={`${i}-${segment.text}`}
            style={color ? { color } : undefined}
            onPress={tappable ? () => onLink?.(segment) : undefined}
          >
            {segment.text}
          </Text>
        );
      })}
    </>
  );
}

/** One chat line: sender (name, tag, claimed check) then the message; the same row serves list and panel. */
export function ChatRow({
  line,
  profile,
  ctx,
  reduce = false,
  onLink,
}: {
  line: ChatLine;
  profile?: ChatProfile;
  ctx: TokenizeContext;
  /** Notification mode: long messages are cut at 80 characters. */
  reduce?: boolean;
  onLink?: (segment: Segment) => void;
}) {
  const kind = rowKind(line, profile);
  const text = reduce ? truncateReduced(line.message) : line.message;
  const mentionsMe = kind !== 'system' && tokenize(text, ctx).some(s => s.mine);
  const nick = profile
    ? { nickname: profile.name, tag: profile.tag, claimed: profile.claimed }
    : splitName(t('COMMON_UNKNOWN_USER'));
  const nameColor =
    kind === 'system'
      ? SYSTEM_GREEN
      : kind === 'own'
      ? colors.mythic
      : profile?.color ?? colors.white70;
  return (
    <View
      style={[
        styles.row,
        { backgroundColor: background[kind] },
        mentionsMe && styles.mention,
      ]}
    >
      <Text style={styles.text}>
        <Text style={[styles.name, { color: nameColor }]}>
          {kind === 'system' ? t('CHAT_SYSTEM_SENDER') : nick.nickname}
        </Text>
        {kind !== 'system' && nick.tag ? (
          <Text style={styles.tag}> {nick.tag}</Text>
        ) : null}
        {kind !== 'system' && nick.claimed && profile ? (
          <Text>
            {' '}
            <Icon name="seal" size={9} color={colors.mythic} />
          </Text>
        ) : null}
      </Text>
      <Text style={[styles.text, styles.message]}>
        {kind === 'system' ? (
          parseBbcode(text).map((run, i) => (
            <Text
              key={i}
              style={[
                run.color ? { color: run.color } : null,
                run.bold ? styles.bold : null,
              ]}
            >
              {run.text}
            </Text>
          ))
        ) : (
          <Body segments={tokenize(text, ctx)} onLink={onLink} />
        )}
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  row: {
    flexDirection: 'row',
    gap: u(2.7),
    alignSelf: 'flex-start',
    maxWidth: u(168.5),
    paddingHorizontal: u(3.3),
    paddingVertical: u(2.7),
    borderRadius: u(2.7),
  },
  mention: { borderWidth: 2, borderColor: MY_MENTION },
  text: {
    fontFamily: fonts.medium,
    fontSize: u(9.33),
    lineHeight: u(11.5),
    color: colors.snow,
  },
  message: { flexShrink: 1 },
  name: { fontFamily: fonts.semibold },
  tag: { color: 'rgba(255,255,255,0.6)' },
  bold: { fontFamily: fonts.semibold },
});
