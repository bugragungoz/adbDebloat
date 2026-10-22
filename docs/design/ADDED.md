# What adbDebloat adds to Bugra

Bugra was drawn for a website. adbDebloat reuses its tokens, fonts and components (copied from the
owner's component-inventory app through netSwitch) and adds the pieces a package manager for a phone
needs. Each one is built from Bugra tokens and shapes and is marked "(added)" in the code. No color
is added: every color comes from `src/styles/tokens.css`, which `npm run check:colors` enforces.

## Components

| Piece | Where | Notes |
|---|---|---|
| Button `quiet` and `danger` | `components/Button.tsx` | Quiet has no outline until hovered; danger uses the `warn` outline and text. |
| Checkbox | `components/Switch.tsx` | Native input, 3:1 outline, accent fill with a check; an indeterminate state for "some selected". |
| Badge | `components.css` | A small outlined pill. `badge-ok`, `badge-info` and `badge-warn` use the status tokens and always carry a word and an icon, never color alone. |
| Chip check | `components.css` | A chosen filter chip shows a check drawn by CSS with empty alternative text, so a screen reader hears the chip's name and "pressed", not "check mark". |
| Safety badge | `components/Safety.tsx` | Recommended (ok, check), Advanced (info, info), Expert (warn, alert), Unsafe (warn, lock), Unknown (neutral). |
| Segmented control | `components/Misc.tsx`, `.tabs` | A pill group with `role="radiogroup"`; the chosen option wears the accent tint. |
| Text field | `components/Misc.tsx` | Bugra's field with a label above and a hint below; monospace for addresses and paths. |
| Meter | `components/Misc.tsx` | A thin accent bar with `role="progressbar"`; the text beside it carries the value. |
| Empty state | `components/Misc.tsx` | Icon, sentence and optional actions, centered in the view. |
| List dialog | `components/Dialog.tsx`, `.bg-dialog-list` | Bugra's dialog at 760 px, as tall as its content: the review before a batch and its results. |
| Snackbar stack | `components/Snackbars.tsx` | Bugra's snackbar, stacked at the bottom left with a close button. |
| Selection bar | `layout.css` | A bordered bar under the table while apps are selected, with the actions and their counts. |
| Detail panel | `layout.css` | Sits beside the table, floats over it (with the floating-layer shadow) below 1180 px. |
| Status bar | `App.tsx`, `layout.css` | adb, the phone, the package list in use, the version and the GitHub link. |

## Icons (`components/icons.ts`)

Bugra-style line icons on a 24 px grid, 1.6 px stroke with round caps. adbDebloat adds `phone`,
`usb`, `wifi`, `trash`, `ban`, `restore`, `history`, `download`, `refresh`, `power`,
`lock` and `unlink`, the mark beside the name in the sidebar (`App.tsx`), and the GitHub mark from
Primer Octicons (MIT) for the link in the status bar.

The app icon (`docs/design/icon.svg`, rendered into `src-tauri/icons`) is the sidebar mark in the
accent on the dark background.
