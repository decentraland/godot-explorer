import {
  Material,
  MeshRenderer,
  Transform,
  VideoPlayer,
  engine,
  type Entity,
  VideoEvent,
  type PBVideoEvent,
  VideoState
} from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'

import { test } from 'testing-library/src/testing'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'

function getVideoState(entity: Entity): PBVideoEvent | undefined {
  let videoState
  for (const event of VideoEvent.get(entity).values()) {
    if (videoState === undefined) {
      videoState = event
    } else if (videoState.timestamp < event.timestamp) {
      videoState = event
    }
  }
  return videoState
}

const TIMEOUT_MS = 100_000

async function waitTimeAndAssertSnapshot(
  screenEntity: Entity,
  t: number,
  snapshotId: number
): Promise<void> {
  const startAt = Date.now()
  await new Promise<void>((resolve, reject) => {
    let lastTickNumber = -1
    let lastEventTime = -1
    let lastEventOffset = -1

    function snapshotSystem(): void {
      const videoState = getVideoState(screenEntity)
      if (Date.now() > startAt + TIMEOUT_MS) {
        engine.removeSystem(systemId)
        reject(new Error(`Timeout waiting for video to reach ${t}`))
        return
      }

      if (videoState === undefined) return
      if (videoState.timestamp === undefined) return
      if (videoState.state !== VideoState.VS_PLAYING) return

      if (videoState.tickNumber > lastTickNumber) {
        lastTickNumber = videoState.tickNumber
        lastEventTime = Date.now()
        lastEventOffset = videoState.currentOffset
      }

      const currentOffset =
        lastEventOffset + (Date.now() - lastEventTime) / 1000

      if (currentOffset >= t && currentOffset < t + 0.5) {
        engine.removeSystem(systemId)
        console.log(
          `Taking snapshot ${snapshotId} with video at ${currentOffset}`
        )
        assertSnapshot(
          `screenshot/$explorer_snapshot_video_player_${snapshotId}.png`,
          Vector3.create(8, 8, 8),
          Vector3.create(8, 8, 16)
        )
          .then(() => {
            resolve()
          })
          .catch(reject)
      }
    }

    const systemId = `video-snapshot-${Math.floor(Math.random() * 1000000)}`
    engine.addSystem(snapshotSystem, 0, systemId)
  })
}

test('video-player: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const screenEntity = customAddEntity.addEntity()
  MeshRenderer.createOrReplace(screenEntity, {
    mesh: { $case: 'plane', plane: { uvs: [] } }
  })
  Transform.create(screenEntity, {
    position: Vector3.create(8, 8, 15),
    scale: Vector3.create(16, 16, 1)
  })
  VideoPlayer.create(screenEntity, {
    src: 'src/assets/videos/dae-video-1.mp4',
    playing: true,
    volume: 0.2,
    loop: true,
    position: 0
  })

  const videoTexture = Material.Texture.Video({
    videoPlayerEntity: screenEntity
  })

  Material.setPbrMaterial(screenEntity, {
    texture: videoTexture,
    roughness: 1.0,
    specularIntensity: 0,
    metallic: 0
  })

  await waitTimeAndAssertSnapshot(screenEntity, 0.25, 1)
  await waitTimeAndAssertSnapshot(screenEntity, 1.25, 2)
  await waitTimeAndAssertSnapshot(screenEntity, 2.25, 3)
  await waitTimeAndAssertSnapshot(screenEntity, 3.25, 4)
})
