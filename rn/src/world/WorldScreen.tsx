import { ComponentRef, useEffect, useRef, useState } from 'react';
import {
  ActivityIndicator,
  Image,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  TextInput,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { Place } from '../discover/api';
import { AppText, colors, LinearFill, u } from '../ds';
import { send, useEngine } from '../engine';

const LOADING_TIMEOUT_MS = 25_000;

function HudButton({
  label,
  accent,
  onPress,
}: {
  label: string;
  accent?: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        accent && styles.accent,
        pressed && styles.pressed,
      ]}
    >
      <AppText variant="button" style={styles.buttonText}>
        {label}
      </AppText>
    </Pressable>
  );
}

/**
 * The world as RN sees it: Godot draws the 3D scene under this view; the cover hides the
 * boot until the explorer reports it is playing. Scene UI, joystick and camera stay native.
 */
export function WorldScreen({
  place,
  onLeave,
}: {
  place: Place;
  onLeave: () => void;
}) {
  const insets = useSafeAreaInsets();
  const { hud } = useEngine();
  const [chatOpen, setChatOpen] = useState(false);
  const [draft, setDraft] = useState('');
  const [timedOut, setTimedOut] = useState(false);
  const chatScroll = useRef<ComponentRef<typeof ScrollView>>(null);

  useEffect(() => {
    const timer = setTimeout(() => setTimedOut(true), LOADING_TIMEOUT_MS);
    return () => clearTimeout(timer);
  }, []);

  const covered = hud.phase !== 'playing' && !timedOut;
  const parcel = hud.parcel
    ? `${hud.parcel[0]},${hud.parcel[1]}`
    : place.location;

  const submit = () => {
    const text = draft.trim();
    if (text) {
      send({ type: 'send_chat', text });
    }
    setDraft('');
  };

  return (
    <View style={styles.root} pointerEvents="box-none">
      <View
        style={[styles.top, { paddingTop: insets.top + u(12) }]}
        pointerEvents="box-none"
      >
        <View style={styles.chip} pointerEvents="none">
          <AppText variant="caption" numberOfLines={1} style={styles.chipText}>
            {place.title} · {parcel}
          </AppText>
        </View>
        <View style={styles.actions} pointerEvents="box-none">
          <HudButton
            label="Wave"
            onPress={() => send({ type: 'emote', urn: 'wave' })}
          />
          <HudButton
            label={chatOpen ? 'Close' : 'Chat'}
            onPress={() => setChatOpen(open => !open)}
          />
          <HudButton label="Leave" accent onPress={onLeave} />
        </View>
      </View>
      {chatOpen && (
        <KeyboardAvoidingView
          style={[styles.chat, { top: insets.top + u(64) }]}
          behavior={Platform.OS === 'ios' ? 'padding' : undefined}
        >
          <ScrollView
            ref={chatScroll}
            style={styles.chatLog}
            onContentSizeChange={() =>
              chatScroll.current?.scrollToEnd({ animated: false })
            }
          >
            {hud.chat.map((line, index) => (
              <AppText
                key={`${line.timestamp}-${index}`}
                variant="caption"
                style={styles.chatLine}
              >
                <AppText variant="caption" style={styles.chatAuthor}>
                  {line.address.slice(0, 6)}{' '}
                </AppText>
                {line.message}
              </AppText>
            ))}
          </ScrollView>
          <TextInput
            style={styles.chatInput}
            value={draft}
            onChangeText={setDraft}
            onSubmitEditing={submit}
            placeholder="Say something…"
            placeholderTextColor={colors.silver}
            returnKeyType="send"
          />
        </KeyboardAvoidingView>
      )}
      {covered && (
        <View style={styles.cover}>
          <LinearFill
            stops={[
              { offset: 0, color: colors.purple450 },
              { offset: 1, color: colors.purple550 },
            ]}
          />
          {place.image ? (
            <Image
              source={{ uri: place.image }}
              style={styles.coverImage}
              resizeMode="cover"
            />
          ) : null}
          <AppText variant="heading" style={styles.coverTitle}>
            {place.title}
          </AppText>
          <View style={styles.coverRow}>
            <ActivityIndicator color={colors.snow} />
            <AppText variant="body" style={styles.coverText}>
              Jumping in…
            </AppText>
          </View>
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: 'transparent' },
  top: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingHorizontal: u(16),
  },
  chip: {
    flexShrink: 1,
    marginRight: u(8),
    backgroundColor: 'rgba(0,0,0,0.55)',
    borderRadius: u(12),
    paddingHorizontal: u(12),
    paddingVertical: u(6),
  },
  chipText: { color: colors.snow },
  actions: { flexDirection: 'row', gap: u(8) },
  button: {
    backgroundColor: 'rgba(22,21,24,0.8)',
    borderRadius: 100,
    paddingHorizontal: u(14),
    paddingVertical: u(8),
  },
  accent: { backgroundColor: colors.ruby },
  buttonText: { fontSize: u(13) },
  pressed: { opacity: 0.8 },
  chat: {
    position: 'absolute',
    left: u(16),
    right: u(16),
    maxWidth: u(320),
    backgroundColor: 'rgba(0,0,0,0.6)',
    borderRadius: u(12),
    padding: u(8),
  },
  chatLog: { maxHeight: u(140) },
  chatLine: { color: colors.snow, marginBottom: u(2) },
  chatAuthor: { color: '#ffb347' },
  chatInput: {
    marginTop: u(6),
    backgroundColor: 'rgba(255,255,255,0.15)',
    borderRadius: u(8),
    color: colors.snow,
    paddingHorizontal: u(10),
    paddingVertical: u(6),
  },
  cover: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
  },
  coverImage: {
    width: '80%',
    aspectRatio: 1.9,
    borderRadius: u(6),
    marginBottom: u(24),
  },
  coverTitle: { textAlign: 'center', paddingHorizontal: u(24) },
  coverRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(10),
    marginTop: u(12),
  },
  coverText: { color: colors.gray },
});
