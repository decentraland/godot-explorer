import ReactEcs, { Label, ReactEcsRenderer, UiEntity } from '@dcl/sdk/react-ecs'
import { Color4 } from '@dcl/sdk/math'

/**
 * A production build (the one deployed to the QA World) is the QA mode: `test()` does nothing
 * there, and the scene shows its manual steps instead. CI serves development builds.
 */
export const isQaMode = !DEBUG

const NO_MANUAL_STEPS = 'Automated checks only: nothing to test by hand here.'

/** Shows the scene's manual QA steps in QA mode; does nothing under the test runner. */
export function showQaSteps(title: string, steps: string[]): void {
  if (!isQaMode) return
  let open = true
  const lines = steps.length > 0 ? steps.map((step, index) => `${index + 1}. ${step}`) : [NO_MANUAL_STEPS]

  ReactEcsRenderer.setUiRenderer(() => (
    <UiEntity
      uiTransform={{
        positionType: 'absolute',
        position: { top: '12%', left: '3%' },
        width: '40%',
        flexDirection: 'column',
        padding: 16
      }}
      uiBackground={{ color: Color4.create(0, 0, 0, 0.75) }}
    >
      <UiEntity
        uiTransform={{ width: '100%', flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' }}
      >
        <Label value={`QA: ${title}`} fontSize={20} color={Color4.White()} textAlign="middle-left" />
        <Label
          value={open ? 'Hide' : 'Show'}
          fontSize={16}
          color={Color4.Yellow()}
          uiTransform={{ padding: 8 }}
          onMouseDown={() => {
            open = !open
          }}
        />
      </UiEntity>
      {open
        ? lines.map((line, index) => (
            <Label
              key={index}
              value={line}
              fontSize={16}
              color={Color4.White()}
              textAlign="top-left"
              textWrap="wrap"
              uiTransform={{ width: '100%', margin: { top: 8 } }}
            />
          ))
        : null}
    </UiEntity>
  ))
}
