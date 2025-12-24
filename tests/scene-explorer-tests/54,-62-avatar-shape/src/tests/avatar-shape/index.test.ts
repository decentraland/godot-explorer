import { Animator, AvatarShape, Transform } from '@dcl/sdk/ecs'
import { Color3, Vector3 } from '@dcl/sdk/math'
import { test } from 'testing-library/src/testing'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'

test('avatar-shape: title', async function (context) {
  const avatarEntity = customAddEntity.addEntity()
  AvatarShape.create(avatarEntity, {
    id: 'test',
    emotes: [],
    hairColor: Color3.Blue(),
    skinColor: Color3.Green(),
    eyeColor: Color3.Red(),
    bodyShape: 'urn:decentraland:off-chain:base-avatars:BaseFemale',
    wearables: [
      'urn:decentraland:off-chain:base-avatars:eyebrows_00',
      'urn:decentraland:off-chain:base-avatars:f_eyes_04',
      'urn:decentraland:off-chain:base-avatars:f_mouth_00',
      'urn:decentraland:off-chain:base-avatars:double_bun',
      'urn:decentraland:matic:collections-v2:0x305a98adbc3a78e482bb2d935da5401d6528d855:2',
      'urn:decentraland:matic:collections-v2:0x9a9949bb49170fcec4891f4cfa7370281acb8cae:2',
      'urn:decentraland:matic:collections-v2:0x99fa3feb28b30472fd20094a58f9a2f1ffafa3f7:0',
      'urn:decentraland:matic:collections-v2:0x4cd42861d69309d7b6f01cfd40dce9bbb8bf8a81:0',
      'urn:decentraland:matic:collections-v2:0xb366c9c59ace21c18b7d03e179c980bff979f5f4:0',
      'urn:decentraland:matic:collections-v2:0x9e43dc87a0166e1536d770c55b624cf7cd45c442:0',
      'urn:decentraland:matic:collections-v2:0x83d431a9a5084bf26ef4e1081e26fbe90798aa3a:0'
    ]
  })

  Animator.stopAllAnimations(avatarEntity)

  Transform.create(avatarEntity, {
    position: Vector3.create(8, 0.25, 8)
  })

  await context.helpers.waitNTicks(500)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_avatar_shape.png',
    Vector3.create(8, 2.5, 9.5),
    Vector3.create(8, 1, 8)
  )
})
