import { type TestFunctionContext } from '../testing/types'

export async function waitForMeshColliderApplied(
  context: TestFunctionContext
): Promise<void> {
  // TODO: (GODOT) review this issue in godot, the physics server doesn't add immediately the mesh colider
  await context.helpers.waitNTicks(3)

  // TODO: (GODOT) review this issue in godot, the raycast sent is not hitting the two cubes
  //  lean thinks that it's because the Physics Server doesn't apply the transform update
  //  until a new physic's tick comes. this waitNTicks  is relative at scene-tick rate
  //  in the godot case the scene has to wait as many ticks as physics tick fits in the scene-rate
}
