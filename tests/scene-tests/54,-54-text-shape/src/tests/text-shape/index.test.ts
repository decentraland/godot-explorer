import { Entity, Font, PBTextShape, TextAlignMode, TextShape, Transform } from '@dcl/sdk/ecs'
import { Color4, Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { renderedText, settled } from 'testing-library/src/utils/engine'
import { test } from 'testing-library/src/testing'
import { TestFunctionContext } from 'testing-library/src/testing/types'
import { assert, assertEquals } from 'testing-library/src/testing/assert'

async function label(context: TestFunctionContext, shape: PBTextShape) {
  customAddEntity.clean()
  const entity: Entity = customAddEntity.addEntity()
  Transform.create(entity, { position: Vector3.create(8, 1, 8) })
  TextShape.createOrReplace(entity, shape)
  return await settled(context, () => renderedText(entity), (l) => l !== null && l.text !== '')
}

type Expected = Partial<Omit<NonNullable<Awaited<ReturnType<typeof renderedText>>>, 'font'>> & { font?: string }

// Label3D alignment: 0 left/top, 1 center, 2 right/bottom. Font sizes are label pixels, and each
// font has its own scale. Values as the explorer draws them on 2026-11-03.
const CASES: Array<[string, PBTextShape, Expected]> = [
  [
    'default',
    { text: 'Default text ' },
    { text: 'Default text', fontSize: 54, color: [1, 1, 1, 1], outlineWidth: 0, horizontalAlignment: 1, verticalAlignment: 1, font: 'Inter-SemiBold.ttf' }
  ],
  ['colorized', { text: 'Red text', fontSize: 3, textColor: Color4.Red() }, { text: 'Red text', fontSize: 54, color: [1, 0, 0, 1] }],
  [
    'outlined',
    { text: 'Text with\nred outline', fontSize: 2, outlineColor: Color4.Red(), outlineWidth: 0.1 },
    { text: 'Text with\nred outline', fontSize: 36, outlineColor: [1, 0, 0, 1], outlineWidth: 0.073 }
  ],
  ['monospace', { text: 'Monospace', fontSize: 2, font: Font.F_MONOSPACE }, { fontSize: 38, font: 'AtkinsonHyperlegibleMono.ttf' }],
  ['serif', { text: 'Serif', fontSize: 2, font: Font.F_SERIF }, { fontSize: 37, font: 'NotoSerif.ttf' }],
  ['bottom center', { text: 'Bottom Center', fontSize: 2, textAlign: TextAlignMode.TAM_BOTTOM_CENTER }, { horizontalAlignment: 1, verticalAlignment: 2 }],
  ['middle right', { text: 'Middle Right', fontSize: 2, textAlign: TextAlignMode.TAM_MIDDLE_RIGHT }, { horizontalAlignment: 2, verticalAlignment: 1 }],
  ['top left', { text: 'Top Left', fontSize: 2, textAlign: TextAlignMode.TAM_TOP_LEFT }, { horizontalAlignment: 0, verticalAlignment: 0 }]
]

for (const [name, shape, expected] of CASES) {
  test(`text-shape: ${name}`, async function (context) {
    const drawn = await label(context, shape)
    assert(drawn !== null, 'no label')
    const actual = { ...drawn!, font: drawn!.font?.split('/').pop(), outlineWidth: Math.round(drawn!.outlineWidth * 1000) / 1000 }
    for (const [field, value] of Object.entries(expected)) {
      assertEquals(actual[field as keyof typeof actual], value, field)
    }
    customAddEntity.clean()
  })
}

test('text-shape: changing the text updates the label', async function (context) {
  const entity = customAddEntity.addEntity()
  Transform.create(entity, { position: Vector3.create(8, 1, 8) })
  TextShape.create(entity, { text: 'before' })
  await settled(context, () => renderedText(entity), (l) => l?.text === 'before')
  TextShape.getMutable(entity).text = 'after'
  assertEquals((await settled(context, () => renderedText(entity), (l) => l?.text === 'after'))?.text, 'after')
  customAddEntity.clean()
})
