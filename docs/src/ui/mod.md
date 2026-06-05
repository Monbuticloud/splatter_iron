# ui

egui-based UI panels composing the SplatterIron editor interface. Each panel
is implemented as an `impl MyApp` method and is wired into the main `update()`
loop in `MyApp`.

## Submodules

| Module    | Panel / Role             | Purpose                                                                                       |
| --------- | ------------------------ | --------------------------------------------------------------------------------------------- |
| `top`     | Top menu bar             | Save, Load, New, Export, Import, Undo, Redo, Close                                            |
| `left`    | Left tool palette        | Tool selection (Square, Circle, Square Eraser, Circle Eraser, Bucket Fill)                    |
| `right`   | Right properties panel   | Colour picker, brush radius, alpha overlay toggle, layer management, undo strength, save path |
| `center`  | Central canvas           | Texture rendering, brush preview, mouse interaction handling, stroke application              |
| `dialogs` | Dialog windows           | Error list, confirmations (delete, large canvas), stamp/brush naming, toast, progress         |
| `panels`  | Panel layout coordinator | Dispatches to top/left/right/centre panel methods                                             |

### Panel layout

The four panels are arranged via egui's `TopBottomPanel`, `SidePanel`, and
`CentralPanel` layout system:

```
┌──────────────┬──────────────────────────────────┬──────────────┐
│              │           Top Panel               │              │
│              │  (Save, Load, Export, Undo, …)    │              │
│   Left       │                                  │   Right      │
│   Panel      │         Central Panel             │   Panel      │
│  (Tools)     │       (Canvas + Preview)          │  (Color,     │
│              │                                  │   Layers,    │
│              │                                  │   Radius)    │
│              │                                  │              │
└──────────────┴──────────────────────────────────┴──────────────┘
```
