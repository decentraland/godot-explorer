import { useRef, useState } from 'react';
import { FlatList, StyleSheet, View, useWindowDimensions } from 'react-native';
import { AppText, colors, PrimaryButton } from '../ds';
import { FlowFrame } from './FlowFrame';

const SLIDES = [
  {
    title: 'Explore a living world',
    body: 'Wander through scenes built by creators from all over the world.',
  },
  {
    title: 'Make it yours',
    body: 'Dress your avatar and pick the emotes that say who you are.',
  },
  {
    title: 'Meet people',
    body: 'Chat, dance and hang out with whoever is around you.',
  },
];

/** First-time-user tour: three swipeable slides ending in "Let's go". */
export function FtueScreen({ onDone }: { onDone: () => void }) {
  const { width } = useWindowDimensions();
  const pageWidth = width - 48;
  const list = useRef<FlatList<(typeof SLIDES)[number]>>(null);
  const [index, setIndex] = useState(0);
  const last = index === SLIDES.length - 1;

  const next = () => {
    if (last) {
      onDone();
    } else {
      list.current?.scrollToIndex({ index: index + 1, animated: true });
    }
  };

  return (
    <FlowFrame>
      <FlatList
        ref={list}
        data={SLIDES}
        horizontal
        pagingEnabled
        showsHorizontalScrollIndicator={false}
        keyExtractor={slide => slide.title}
        onMomentumScrollEnd={event =>
          setIndex(Math.round(event.nativeEvent.contentOffset.x / pageWidth))
        }
        getItemLayout={(_data, itemIndex) => ({
          length: pageWidth,
          offset: pageWidth * itemIndex,
          index: itemIndex,
        })}
        renderItem={({ item }) => (
          <View style={[styles.slide, { width: pageWidth }]}>
            <AppText variant="heading" style={styles.title}>
              {item.title}
            </AppText>
            <AppText variant="body" style={styles.body}>
              {item.body}
            </AppText>
          </View>
        )}
      />
      <View style={styles.dots}>
        {SLIDES.map((slide, dot) => (
          <View
            key={slide.title}
            style={[styles.dot, dot === index && styles.dotOn]}
          />
        ))}
      </View>
      <View style={styles.actions}>
        <PrimaryButton label={last ? "Let's go" : 'Next'} onPress={next} />
        {!last && <PrimaryButton label="Skip" tone="ghost" onPress={onDone} />}
      </View>
    </FlowFrame>
  );
}

const styles = StyleSheet.create({
  slide: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 12,
  },
  title: { textAlign: 'center' },
  body: { marginTop: 12, textAlign: 'center', opacity: 0.85 },
  dots: {
    flexDirection: 'row',
    justifyContent: 'center',
    gap: 8,
    marginVertical: 20,
  },
  dot: {
    width: 8,
    height: 8,
    borderRadius: 4,
    backgroundColor: colors.white30,
  },
  dotOn: { backgroundColor: colors.ruby, width: 20 },
  actions: { gap: 12, minHeight: 108 },
});
