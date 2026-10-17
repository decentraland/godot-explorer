# UiInput / soft keyboard test scene (plan 2.8, risks R-3)

A minimal SDK7 scene with two `Input`s (one near the top, one at the bottom where the keyboard overlaps), a
label that mirrors the value and the last submit. Not built or run in CI: it exists to be loaded under the RN
overlay on a phone.

```bash
cd tests/ime-uiinput-scene
npm install
npx @dcl/sdk-commands start --port 8000      # then reach it from the phone, e.g. http://<LAN-IP>:8000
```

Enter it with the emulator on desktop (`cargo run -- run -- --preview http://localhost:8000`) to see it work first,
then in the RN host: `enter_world` with that realm (or the deeplink `decentraland://open?preview=http://<LAN-IP>:8000`).

## Checklist, per phone (Android and iOS), RN host

- [ ] Tapping the bottom input focuses it and the keyboard appears.
- [ ] Godot reports a keyboard height > 0 (`DisplayServer.virtual_keyboard_get_height()`, the `focus` tracker of the
      debug hub shows the focused control). On Android the RN host passes `--force-mobile`, which skips the
      `global.gd` height poll — note whether anything depends on it.
- [ ] Typing reaches the scene: the label shows `value: …` and updates per character.
- [ ] Submit (the keyboard's action key) sets `submitted: …`.
- [ ] Dismissing the keyboard resets the height to 0 and unfocuses the input.
- [ ] The input stays visible above the keyboard (or the scene UI shifts); the top input is unaffected.
- [ ] Rotating while the keyboard is open does not lose focus or the text.
- [ ] Focusing an RN `TextInput` (e.g. chat) while a scene input is focused does not leave two keyboards.
- [ ] Touch outside RN views reaches the scene input (RN overlay has a transparent hole there).

Record the result (pass/fail per row, device, OS version, build commit) in `plan/risks.md` R-3.
