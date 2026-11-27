import { getExplorerInformation } from '~system/Runtime'

/** The two languages the explorer reports: the app's (`locale`) and the device's (`systemLocale`). */
export async function readLocales(): Promise<{
  locale: string
  systemLocale: string
}> {
  const info = await getExplorerInformation({})
  return {
    locale: String(info.configurations?.locale ?? ''),
    systemLocale: String(info.configurations?.systemLocale ?? '')
  }
}
