# @dcl/ui-native

The Decentraland mobile design system for React Native: tokens, text styles, icons and components,
built from the Figma file
[Design System Mobile](https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile).
It is an npm workspace of `rn/`; the app imports it as `@dcl/ui-native` (no build step, `main` is
`src/index.ts`).

```tsx
import { AppText, Button, color, space, u } from '@dcl/ui-native';
```

## Rules

- **Use a component before writing one.** Every component's doc comment links to its Figma node; in
  Figma, Dev Mode shows the same component through Code Connect.
- **Sizes go through `u()`.** Figma frames are 720 px wide at 2x, a 360 dp phone. Take the Figma px,
  divide by 2 and wrap it: a 48 px button is `u(24)`. `u()` scales it to the real screen width
  (`SCALE = min(width, height) / 360`). Never use a bare number for a size, except a hairline border
  (`border.thin`).
- **Colors, spacing and radii are tokens, by their Figma name.** Do not write hex or rgba in a
  component. If Figma uses a value that has no token here, add the token with its Figma name, not a
  literal.
- **Text is `AppText` with a Figma text style**: `<AppText variant="Modal/Title">`. Do not set
  `fontFamily` or `fontSize` by hand. A missing style goes into `figmaStyles` in `AppText.tsx`.
- **Icons are `<Icon name=…>`.** Names are typed (`IconName`, `iconNames`). Glyphs live in
  `iconSvgs.ts`, exported from the Figma `Icon/*` components; a new icon is exported there, never
  drawn by hand.
- **No user-facing text inside the package.** Labels come in as props, already translated. The few
  strings a component writes itself (accessibility labels such as close or clear) are
  `UiNativeKey`s in `i18n.ts`; the app installs its translator with `setTranslator`
  (`rn/src/i18n/index.ts`).

## Tokens (`tokens.ts`)

| Export | Figma | Example |
|---|---|---|
| `color` | the semantic **Color** collection | `color.text.primary`, `color.surface.modal`, `color.icon.hud` |
| `colors` | the primitive palette | `colors.ruby`, `colors.lavender`, `colors.purple500` |
| `alpha` | the `alpha/*` primitives | `alpha['white-30']`, `alpha['purple-70']` |
| `space` | Spacing & Sizes | `space.md` (16 px in Figma), `space['40']` (the Figma `space/40`) |
| `radius` | radii | `radius.lg`, `radius.pill` |
| `control`, `icon` | the **Size** collection | `control.lg` (button height), `icon['button-md']` |
| `elevation` | `Elevation/Down`, `Up`, `Menu` | `...elevation.down` in a style |
| `motion` | the Motion page | `motion.duration.base`, `motion.easing.standard` |
| `fonts`, `layout`, `rarityColors`, `lightRainbow` | fonts, item grid, rarity and gradient fills | |

Prefer `color.*` (what the value is for) over `colors.*` (what it is). `controlHeight` and `iconSize`
are deprecated aliases of `control` and `icon`; `radii` is the older name set for `radius`.

## Components

Grouped by what they are for; the doc comment in each file says which Figma component it matches.

- **Text and layout**: `AppText`, `BrandBackground`, `GradientView`, `LinearFill`, `RadialFill`,
  `ScrollFade`, `Header`, `SectionCollapsible`
- **Buttons**: `Button` (tones: primary, secondary, ghost, dark, outline, reminder, claim), `IconButton`, `PillButton`,
  `LinkButton`, `LoginButton`, `ActionSquareButton`, `HudCircleButton`, `ButtonProfileLink`,
  `ButtonSeeMore`
- **Inputs**: `TextField`, `SearchField`, `InputUsername`, `Dropdown`, `Checkbox`, `Switch`,
  `Slider`, `CodeSlots`, `HsvSliders`, `Swatch`, `SwatchRow`
- **Tags and badges**: `Badge`, `NotificationBadge`, `Chip`, `TagCategory`, `EventBadge`,
  `EventTimeTag`, `RarityLabel`, `ProfileAttribute`
- **Cards and rows**: `ItemCard`, `SkeletonCard`, `RarityBackground`, `DiscoverCard`,
  `DiscoverCardSkeleton`, `DetailRow`, `AvatarCard`, `ProfileCard`, `SlotTile`, `ScreenshotCard`,
  `FriendRow`, `NotificationRow`, `LinkRow`, `SettingsRow`
- **Navigation**: `BottomNav`, `Tabs`, `MenuNavItem`, `MenuDrawer`, `Menu`, `SnapCarousel`, `PageDots`
- **Dialogs and sheets**: `Dialog`, `ModalSpecial`, `ClaimDialog`, `OtpDialog`, `BugReportDialog`,
  `PlaceModalLandscape`, `Sheet`, `SheetFooter`, `BottomSheet`
- **Feedback**: `ToastProvider` + `useToast`, `WarningBanner`, `EmptyState`, `ProgressBar`,
  `Skeleton`, `Spinner`

## Adding or changing a component

1. Build it in Figma first, in the Design System Mobile file, with tokens and text styles.
2. Add `src/<Name>.tsx`: tokens and `u()` only, props for every string, and a doc comment with one
   line on what it is and the Figma link
   (`Figma <Component>: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/...?node-id=…`).
3. Export it from `src/index.ts`.
4. Add a test under `rn/__tests__/` (the `ds-*.test.tsx` files) and, for something visible, a frame in
   the gallery (`rn/src/gallery/`, then `npm run gallery:ids`).
5. Connect it in Figma with Code Connect so Dev Mode points at the file.

## Checking your work

From `rn/`: `npx tsc --noEmit`, `npx eslint App.tsx src __tests__ packages`, `npx jest`. To see a component, run
`npx expo start --web` and open `/?preview=<gallery id>` (for example `extra/ds-buttons`); the gallery
ids are in `rn/src/gallery/ids.json`. Visual snapshots in CI are described in `rn/README.md`.
