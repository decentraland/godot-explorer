/**
 * VIBE-CODED SPIKE: React Native HUD drawn over the embedded Godot 3D view in world mode. Not
 * mounted while Godot only renders the avatar preview; it returns with the explore step.
 * Scene UI, joystick and gamepad stay native in Godot; everything else is here.
 */
import { ComponentRef, useEffect, useReducer, useRef, useState } from 'react';
import {
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StatusBar,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { sendCommand, subscribe } from './GodotBridge';
import { GodotEvent, HudState, initialHud, reduceHud } from './events';

const LOADING_TIMEOUT_MS = 25_000;

export function WorldHud() {
  const [hud, dispatch] = useReducer(reduceHud, initialHud);
  const [chatOpen, setChatOpen] = useState(false);
  const [draft, setDraft] = useState('');
  const [timedOut, setTimedOut] = useState(false);
  const chatScroll = useRef<ComponentRef<typeof ScrollView>>(null);

  useEffect(() => subscribe((event: GodotEvent) => dispatch(event)), []);

  useEffect(() => {
    if (hud.phase !== 'loading') {
      return undefined;
    }
    const timer = setTimeout(() => setTimedOut(true), LOADING_TIMEOUT_MS);
    return () => clearTimeout(timer);
  }, [hud.phase]);

  const covered =
    hud.phase === 'booting' || (hud.phase === 'loading' && !timedOut);

  const submit = () => {
    const text = draft.trim();
    if (text) {
      sendCommand({ type: 'send_chat', text });
    }
    setDraft('');
  };

  return (
    <View style={styles.root} pointerEvents="box-none">
      <StatusBar hidden />
      <StatusChip hud={hud} />
      <View style={styles.actions} pointerEvents="box-none">
        <ActionButton
          label="Wave"
          onPress={() => sendCommand({ type: 'emote', urn: 'wave' })}
        />
        <ActionButton
          label="Plaza"
          onPress={() => sendCommand({ type: 'teleport', x: 0, y: 0 })}
        />
        <ActionButton
          label={chatOpen ? 'Close' : 'Chat'}
          onPress={() => setChatOpen(open => !open)}
        />
      </View>
      {chatOpen && (
        <KeyboardAvoidingView
          style={styles.chat}
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
              <Text key={`${line.timestamp}-${index}`} style={styles.chatLine}>
                <Text style={styles.chatAuthor}>
                  {line.address.slice(0, 6)}{' '}
                </Text>
                {line.message}
              </Text>
            ))}
          </ScrollView>
          <TextInput
            style={styles.chatInput}
            value={draft}
            onChangeText={setDraft}
            onSubmitEditing={submit}
            placeholder="Say something…"
            placeholderTextColor="#999"
            returnKeyType="send"
          />
        </KeyboardAvoidingView>
      )}
      {covered && <LoadingCover phase={hud.phase} />}
    </View>
  );
}

function StatusChip({ hud }: { hud: HudState }) {
  const parcel = hud.parcel ? `${hud.parcel[0]},${hud.parcel[1]}` : '-';
  const position = hud.position
    ? `${hud.position[0].toFixed(1)}, ${hud.position[2].toFixed(1)}`
    : '-';
  return (
    <View style={styles.chip} pointerEvents="none">
      <Text style={styles.chipText}>
        Parcel {parcel} · {position}
      </Text>
    </View>
  );
}

function ActionButton({
  label,
  onPress,
}: {
  label: string;
  onPress: () => void;
}) {
  return (
    <Pressable style={styles.button} onPress={onPress}>
      <Text style={styles.buttonText}>{label}</Text>
    </Pressable>
  );
}

function LoadingCover({ phase }: { phase: HudState['phase'] }) {
  return (
    <View style={styles.cover}>
      <Text style={styles.coverTitle}>Decentraland</Text>
      <Text style={styles.coverText}>
        {phase === 'booting' ? 'Starting engine…' : 'Loading Genesis Plaza…'}
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: 'transparent' },
  chip: {
    position: 'absolute',
    top: 16,
    left: 48,
    backgroundColor: 'rgba(0,0,0,0.55)',
    borderRadius: 12,
    paddingHorizontal: 12,
    paddingVertical: 6,
  },
  chipText: { color: '#fff', fontSize: 13 },
  actions: {
    position: 'absolute',
    top: 16,
    right: 140,
    flexDirection: 'row',
    gap: 8,
  },
  button: {
    backgroundColor: 'rgba(255,45,85,0.9)',
    borderRadius: 18,
    paddingHorizontal: 16,
    paddingVertical: 8,
  },
  buttonText: { color: '#fff', fontWeight: '600' },
  chat: {
    position: 'absolute',
    top: 64,
    left: 48,
    width: 320,
    backgroundColor: 'rgba(0,0,0,0.6)',
    borderRadius: 12,
    padding: 8,
  },
  chatLog: { maxHeight: 140 },
  chatLine: { color: '#fff', fontSize: 13, marginBottom: 2 },
  chatAuthor: { color: '#ffb347', fontWeight: '600' },
  chatInput: {
    marginTop: 6,
    backgroundColor: 'rgba(255,255,255,0.15)',
    borderRadius: 8,
    color: '#fff',
    paddingHorizontal: 10,
    paddingVertical: 6,
  },
  cover: {
    ...StyleSheet.absoluteFill,
    backgroundColor: '#16101f',
    alignItems: 'center',
    justifyContent: 'center',
  },
  coverTitle: { color: '#fff', fontSize: 28, fontWeight: '700' },
  coverText: { color: '#c9b8e8', fontSize: 16, marginTop: 8 },
});
