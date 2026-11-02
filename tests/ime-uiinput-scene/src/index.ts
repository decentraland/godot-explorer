import { ReactEcsRenderer } from '@dcl/sdk/react-ecs'
import { ui } from './ui'

export function main() {
  ReactEcsRenderer.setUiRenderer(ui)
}
