import { test } from 'testing-library/src/testing'
import { assert, assertEquals } from 'testing-library/src/testing/assert'
import { setSetting, settled } from 'testing-library/src/utils/engine'
import { readLocales } from '../../locale'

const BCP47 = /^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$/

test('locale: the app and device languages are BCP-47 tags', async function () {
  const { locale, systemLocale } = await readLocales()
  assert(BCP47.test(locale), `locale is "${locale}"`)
  assert(BCP47.test(systemLocale), `systemLocale is "${systemLocale}"`)
})

test('locale: changing the app language leaves the device language alone', async function (context) {
  const before = await readLocales()
  try {
    for (const [setting, tag] of [
      ['pt_BR', 'pt-BR'],
      ['es', 'es']
    ]) {
      await setSetting('locale', setting)
      const after = await settled(
        context,
        readLocales,
        (value) => value.locale === tag
      )
      assertEquals(after.locale, tag, `app language after choosing ${setting}`)
      assertEquals(
        after.systemLocale,
        before.systemLocale,
        `device language after choosing ${setting}`
      )
    }
  } finally {
    // Every scene runs in the same client: back to the default, following the device.
    await setSetting('locale', '')
  }
})
