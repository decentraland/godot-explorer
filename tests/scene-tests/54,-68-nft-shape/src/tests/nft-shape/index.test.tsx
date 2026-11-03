import { test } from 'testing-library/src/testing'

import { NftShape, Transform } from '@dcl/sdk/ecs'
import { Color3, Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'

const FRAME_STYLES: string[] = [
  'NFT_CLASSIC',
  'NFT_BAROQUE_ORNAMENT',
  'NFT_DIAMOND_ORNAMENT',
  'NFT_MINIMAL_WIDE',
  'NFT_MINIMAL_GREY',
  'NFT_BLOCKY',
  'NFT_GOLD_EDGES',
  'NFT_GOLD_CARVED',
  'NFT_GOLD_WIDE',
  'NFT_GOLD_ROUNDED',
  'NFT_METAL_MEDIUM',
  'NFT_METAL_WIDE',
  'NFT_METAL_SLIM',
  'NFT_METAL_ROUNDED',
  'NFT_PINS',
  'NFT_MINIMAL_BLACK',
  'NFT_MINIMAL_WHITE',
  'NFT_TAPE',
  'NFT_WOOD_SLIM',
  'NFT_WOOD_WIDE',
  'NFT_WOOD_TWIGS',
  'NFT_CANVAS',
  'NFT_NONE'
]

FRAME_STYLES.forEach((value: string, index: number) => {
  test(`nft-shape: frame number ${value}`, async function (context) {
    customAddEntity.clean()
    const nft = customAddEntity.addEntity()

    Transform.create(nft, {
      position: Vector3.create(8, 1.75, 8),
      scale: Vector3.create(5, 5, 1)
    })

    NftShape.create(nft, {
      urn: 'urn:decentraland:ethereum:erc721:0x06012c8cf97bead5deae237070f9587f8e7a266d:558536',
      color: Color3.Black(),
      style: index
    })

    // if index is 0 wait more ticks for ensure nft is loaded
    if (index === 0) {
      await context.helpers.waitNTicks(150)
    } else {
      await context.helpers.waitNTicks(20)
    }

    await assertSnapshot(
      `screenshot/$explorer_snapshot_nft_shape_${value.toLowerCase()}.png`,
      Vector3.create(7.75, 1.75, 5),
      Vector3.create(8, 1.75, 8)
    )
  })
})
