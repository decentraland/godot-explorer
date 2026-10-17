import { StyleSheet, View } from 'react-native';
import { u } from '../ds';
import { ChatRow } from './ChatRow';
import type { TokenizeContext } from './chatModel';
import type { NotificationRow } from './notifications';
import type { ChatProfile } from './types';

/** Closed-chat feed under the buttons; click-through so the world keeps every touch. */
export function NotificationList({
  rows,
  profiles,
  ctx,
}: {
  rows: NotificationRow[];
  profiles: Record<string, ChatProfile>;
  ctx: TokenizeContext;
}) {
  return (
    <View pointerEvents="none" style={styles.list}>
      {rows.map(({ line }) => (
        <ChatRow
          key={line.id}
          line={line}
          profile={profiles[line.address]}
          ctx={ctx}
          reduce
        />
      ))}
    </View>
  );
}

const styles = StyleSheet.create({ list: { gap: u(2.3) } });
