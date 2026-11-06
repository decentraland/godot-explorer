import { MainCamera, Transform, VirtualCamera, engine } from '@dcl/sdk/ecs'
import { Color4 } from '@dcl/sdk/math'
import ReactEcs, { Label, UiEntity } from '@dcl/sdk/react-ecs'
import { movePlayerTo } from '~system/RestrictedActions'

// #2885: a scene camera released in the frame of a teleport, set again the next frame and released
// again. Touch look must keep working afterwards.
let step = -1
let elapsed = 0
let status = 'Not run yet'

function startSwitch(): void {
  step = 0
  elapsed = 0
  status = 'Scene camera on'
}

engine.addSystem((dt) => {
  if (step < 0) return
  elapsed += dt
  const camera = cameraEntity()
  if (step === 0) {
    MainCamera.createOrReplace(engine.CameraEntity, { virtualCameraEntity: camera })
    step = 1
  } else if (step === 1 && elapsed > 1.5) {
    void movePlayerTo({ newRelativePosition: { x: 8, y: 0, z: 4 }, cameraTarget: { x: 8, y: 1, z: 16 } })
    MainCamera.deleteFrom(engine.CameraEntity)
    step = 2
  } else if (step === 2) {
    MainCamera.createOrReplace(engine.CameraEntity, { virtualCameraEntity: camera })
    step = 3
  } else if (step === 3) {
    MainCamera.deleteFrom(engine.CameraEntity)
    status = 'Done: drag to look'
    step = -1
  }
})

let camera: ReturnType<typeof engine.addEntity> | undefined
function cameraEntity() {
  if (camera === undefined) {
    camera = engine.addEntity()
    Transform.create(camera, { position: { x: 8, y: 6, z: 1 }, rotation: { x: 0.26, y: 0, z: 0, w: 0.97 } })
    VirtualCamera.create(camera, { defaultTransition: { transitionMode: VirtualCamera.Transition.Time(0) } })
  }
  return camera
}

export function CameraSwitchRepro(): ReactEcs.JSX.Element {
  return (
    <UiEntity uiTransform={{ flexDirection: 'row', alignItems: 'center', margin: { top: 16 } }}>
      <UiEntity
        uiTransform={{ height: 40, padding: { left: 12, right: 12 }, alignItems: 'center', margin: { right: 12 } }}
        uiBackground={{ color: Color4.Gray() }}
        onMouseDown={startSwitch}
      >
        <Label value="Switch scene camera" fontSize={16} color={Color4.White()} />
      </UiEntity>
      <Label value={status} fontSize={16} color={Color4.White()} />
    </UiEntity>
  )
}
