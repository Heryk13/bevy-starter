# Gamepads: the menus, the actions and the sticks

Status: **built** 2026-11-24, tried with a gamepad faked in code (no gamepad on the PC that
built it): the menus on `main`, walking, looking and the camera on `3d/player`, walking on
`2d/player`. A real gamepad still to try.
Date: 2026-11-24
Bevy: 0.20.0
Roadmap: row 18, part 7 of 8 (`docs/specs/multiplayer-core/design.md` lists them): "gamepad
support, split screen's players 2 and up play on gamepads". Row 1 had left gamepads for later.

## Goal

A player with a gamepad plays the whole game without the keyboard: the d-pad or the left stick
moves through the menus, A presses, B goes back; the left stick walks and the right stick looks
around; a few buttons do what an action's key does.

## Decided

| Question | Decision |
| --- | --- |
| A button for an action | `register_pad(app, action, button)` beside `register_action`, only for the actions a gamepad needs: Pause on Start and B, the camera switch on Y (`3d/player`). `Controls::pressed` and `just_pressed` hear the key or any of the action's buttons, on any gamepad; nothing while a slot listens for a key, as for keys |
| Rebinding a button | Not now: the buttons are fixed, and the Controls page lists keys only. Row 1's design leaves room for it: a button is one more binding of an action, and a column on the page |
| The sticks | `Controls::stick(Gamepad::left_stick)` or `right_stick`: the one pushed furthest of all the first player's gamepads. A genre adds it to its walk keys (held to one long, so a stick pushed halfway walks at half the speed) and, in 3D, turns the view by it at `LOOK_STICK_RAD_PER_S`, as so many pixels of mouse a second |
| The menus | `ui/navigation.rs`: the d-pad, or the left stick pushed past `STICK_PUSH` (once, until it comes back inside `STICK_RELEASE`), moves `InputFocus` to the nearest button that way, a button off to the side costing `SIDEWAYS_WEIGHT` times its distance; A triggers `Activate` on it. Only the buttons of the topmost layer (highest `GlobalZIndex`) are reached, so the focus never slips under a dialog; a layer that opens takes the focus to its top button. The focused button is lit as under the pointer |
| B in the menus | The pause action's button, so it does what Esc does: closes the topmost panel, or the pause menu |
| Another player's gamepad | `GuestPad` on a gamepad: `Controls`, the first player's, does not hear it. A split screen gives it out (`3d/player`) |

## Out of scope

Sliders, switches and choices on the Settings page are not reached by the d-pad: they are not
buttons. Glyphs of the gamepad's buttons in the keys panel and the legends.

## What changed while building it

- **The stick that moves the focus is read in a system of its own**, piped into the one that
  moves it: `Controls` reads `InputFocus` (whether a text field has the keyboard), and the system
  that writes it cannot ask for both (B0002, found at the first run).
