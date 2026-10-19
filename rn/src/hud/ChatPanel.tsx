import { useCallback, useEffect, useRef, useState } from 'react';
import {
  BackHandler,
  Image,
  Keyboard,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { colors, fonts, Icon, u } from '../ds';
import { t } from '../i18n';
import { ChatRow } from './ChatRow';
import { Segment, TokenizeContext } from './chatModel';
import { applyMention, matchProfiles, mentionQuery } from './mentions';
import { MentionList } from './MentionList';
import type { ChatMode } from './hudState';
import { HUD_SIZES } from './layout';
import type { ChatLine, ChatProfile } from './types';

const logo = require('../assets/logo.png');
const noop = () => {};

/** Keyboard height in dp; the panel lifts by it so the input stays visible in landscape. */
function useKeyboardHeight(onHide: () => void): number {
  const [height, setHeight] = useState(0);
  const hideRef = useRef(onHide);
  hideRef.current = onHide;
  useEffect(() => {
    const show = Keyboard.addListener('keyboardDidShow', e =>
      setHeight(e.endCoordinates.height),
    );
    const hide = Keyboard.addListener('keyboardDidHide', () => {
      setHeight(0);
      hideRef.current();
    });
    return () => {
      show.remove();
      hide.remove();
    };
  }, []);
  return height;
}

/** Open chat: header, scrolling history, and a "Write a message" field that expands into the input. */
export function ChatPanel({
  mode,
  lines,
  profiles,
  nearby,
  ctx,
  top,
  left,
  bottom,
  onMode,
  onSend,
  onLink,
}: {
  mode: Exclude<ChatMode, 'closed'>;
  lines: ChatLine[];
  profiles: Record<string, ChatProfile>;
  nearby: ChatProfile[];
  ctx: TokenizeContext;
  top: number;
  left: number;
  bottom: number;
  onMode: (mode: ChatMode) => void;
  onSend: (text: string) => void;
  onLink: (segment: Segment) => void;
}) {
  const writing = mode === 'writing';
  const closeInput = useCallback(() => {
    Keyboard.dismiss();
    onMode('open');
  }, [onMode]);
  const keyboard = useKeyboardHeight(writing ? closeInput : noop);
  const scroll = useRef<ScrollView>(null);
  const atBottom = useRef(true);
  const [behind, setBehind] = useState(false);
  const [text, setText] = useState('');
  const [caret, setCaret] = useState(0);
  const [forced, setForced] = useState<
    { start: number; end: number } | undefined
  >();

  useEffect(() => {
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      onMode(writing ? 'open' : 'closed');
      return true;
    });
    return () => sub.remove();
  }, [writing, onMode]);

  useEffect(() => {
    if (atBottom.current) {
      scroll.current?.scrollToEnd({ animated: false });
    } else {
      setBehind(true);
    }
  }, [lines.length]);

  const query = writing ? mentionQuery(text, caret) : null;
  const suggestions = query === null ? [] : matchProfiles(query, nearby);

  const submit = () => {
    const message = text.trim();
    if (!message) {
      return;
    }
    onSend(message);
    setText('');
    if (message.startsWith('/')) {
      onMode('closed');
    }
  };

  const pick = (name: string) => {
    const next = applyMention(text, caret, name);
    setText(next.text);
    setForced({ start: next.caret, end: next.caret });
  };

  return (
    <View
      style={[
        styles.panel,
        {
          top,
          left,
          bottom: bottom + (writing ? keyboard : 0),
          width: u(HUD_SIZES.chatPanelWidth / 2),
        },
      ]}
    >
      {!writing && (
        <View style={styles.header}>
          <Image source={logo} style={styles.headerLogo} />
          <Text style={styles.headerText}>{t('CHAT_HEADER_NEARBY')}</Text>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('CHAT_CLOSE')}
            onPress={() => onMode('closed')}
            hitSlop={u(6)}
          >
            <Icon name="close" size={10} color={colors.snow} />
          </Pressable>
        </View>
      )}
      <ScrollView
        ref={scroll}
        style={styles.messages}
        contentContainerStyle={styles.messagesContent}
        scrollEventThrottle={64}
        onScroll={e => {
          const { contentOffset, contentSize, layoutMeasurement } =
            e.nativeEvent;
          atBottom.current =
            contentOffset.y + layoutMeasurement.height >=
            contentSize.height - u(8);
          if (atBottom.current) {
            setBehind(false);
          }
        }}
      >
        {lines.map(line => (
          <ChatRow
            key={line.id}
            line={line}
            profile={profiles[line.address]}
            ctx={ctx}
            onLink={onLink}
          />
        ))}
      </ScrollView>
      {behind && (
        <Pressable
          accessibilityRole="button"
          onPress={() => {
            scroll.current?.scrollToEnd({ animated: true });
            setBehind(false);
          }}
          style={styles.newMessages}
        >
          <Text style={styles.newMessagesText}>{t('CHAT_NEW_MESSAGES')}</Text>
        </Pressable>
      )}
      <View style={styles.footer}>
        <MentionList profiles={suggestions} onPick={pick} />
        {writing ? (
          <View style={styles.inputRow}>
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={t('CHAT_CLOSE_INPUT')}
              onPress={closeInput}
              style={styles.closeInput}
            >
              <Icon name="close" size={10} color={colors.snow} />
            </Pressable>
            <TextInput
              autoFocus
              disableFullscreenUI
              value={text}
              onChangeText={setText}
              onSelectionChange={e => {
                setCaret(e.nativeEvent.selection.start);
                setForced(undefined);
              }}
              selection={forced}
              onSubmitEditing={submit}
              placeholder={t('CHAT_WRITE_A_MESSAGE_ELLIPSIS')}
              placeholderTextColor={colors.silver}
              returnKeyType="send"
              blurOnSubmit={false}
              style={styles.input}
            />
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={t('CHAT_SEND')}
              disabled={!text.trim()}
              onPress={submit}
              style={[styles.send, !text.trim() && styles.sendOff]}
            >
              <Icon name="arrowRight" size={11} color={colors.snow} />
            </Pressable>
          </View>
        ) : (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('CHAT_WRITE_A_MESSAGE')}
            onPress={() => onMode('writing')}
            style={styles.write}
          >
            <Text style={styles.writeText}>{t('CHAT_WRITE_A_MESSAGE')}</Text>
          </Pressable>
        )}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  panel: {
    position: 'absolute',
    backgroundColor: 'rgba(0,0,0,0.6)',
    borderRadius: u(4),
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(4),
    height: u(26.5),
    paddingHorizontal: u(5),
    backgroundColor: 'rgba(0,0,0,0.6)',
  },
  headerLogo: { width: u(9), height: u(9), borderRadius: u(4.5) },
  headerText: {
    flex: 1,
    fontFamily: fonts.semibold,
    fontSize: u(8.5),
    color: colors.snow,
    textTransform: 'uppercase',
  },
  messages: { flex: 1 },
  messagesContent: { gap: u(2.3), padding: u(5) },
  newMessages: {
    position: 'absolute',
    alignSelf: 'center',
    bottom: u(46),
    paddingHorizontal: u(8),
    paddingVertical: u(3),
    borderRadius: 100,
    backgroundColor: colors.purple350,
  },
  newMessagesText: {
    fontFamily: fonts.semibold,
    fontSize: u(8),
    color: colors.snow,
  },
  footer: { padding: u(5) },
  write: {
    height: u(24),
    justifyContent: 'center',
    paddingHorizontal: u(6),
    borderRadius: u(3),
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.3)',
    backgroundColor: 'rgba(0,0,0,0.3)',
  },
  writeText: {
    fontFamily: fonts.medium,
    fontSize: u(9.5),
    color: colors.silver,
  },
  inputRow: { flexDirection: 'row', alignItems: 'center', gap: u(4) },
  input: {
    flex: 1,
    height: u(24),
    paddingVertical: 0,
    paddingHorizontal: u(6),
    borderRadius: u(3),
    borderWidth: 1,
    borderColor: colors.lavender70,
    backgroundColor: 'rgba(0,0,0,0.4)',
    fontFamily: fonts.medium,
    fontSize: u(9.5),
    color: colors.snow,
  },
  closeInput: {
    width: u(24),
    height: u(24),
    borderRadius: u(12),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: 'rgba(255,255,255,0.15)',
  },
  send: {
    width: u(24),
    height: u(24),
    borderRadius: u(12),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.ruby,
  },
  sendOff: { opacity: 0.4 },
});
