import ReactEcs, {
  UiEntity,
  type JSX,
  type Key,
  type UiBackgroundProps,
  type UiTransformProps
} from '@dcl/sdk/react-ecs'

export function UiItem(
  props: UiTransformProps & UiBackgroundProps & { key?: Key }
): JSX.Element {
  const {
    display,
    flex,
    justifyContent,
    positionType,
    alignItems,
    alignSelf,
    alignContent,
    flexDirection,
    position,
    padding,
    margin,
    width,
    height,
    minWidth,
    maxWidth,
    minHeight,
    maxHeight,
    flexWrap,
    flexBasis,
    flexGrow,
    flexShrink,
    overflow,
    pointerFilter,

    color,
    textureMode,
    textureSlices,
    uvs,
    avatarTexture,
    texture,

    key,
    ...otherProps
  } = props

  const backgroundProps = {
    color,
    textureMode,
    textureSlices,
    uvs,
    avatarTexture,
    texture
  }

  const withBackground = Object.values(backgroundProps).some(
    (value) => value !== undefined
  )

  const transformProps = {
    display,
    flex,
    justifyContent,
    positionType,
    alignItems,
    alignSelf,
    alignContent,
    flexDirection,
    position,
    padding,
    margin,
    width,
    height,
    minWidth,
    maxWidth,
    minHeight,
    maxHeight,
    flexWrap,
    flexBasis,
    flexGrow,
    flexShrink,
    overflow,
    pointerFilter
  }

  if (withBackground) {
    return (
      <UiEntity
        key={key}
        uiTransform={transformProps}
        uiBackground={backgroundProps}
        {...otherProps}
      />
    )
  } else {
    return <UiEntity key={key} uiTransform={transformProps} {...otherProps} />
  }
}

export function MainCanvas(props: any): JSX.Element {
  return (
    <UiItem
      position={{ left: 0, top: 0 }}
      positionType="absolute"
      height="100%"
      width="100%"
      color={props.color}
    >
      {props.children}
    </UiItem>
  )
}
