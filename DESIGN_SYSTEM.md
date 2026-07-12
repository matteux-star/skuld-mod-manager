# Skuld Mod Manager — Design System

> Extracted from `src/styles/globals.css`. Dark, utilitarian, precise. Function over decoration.
> Built on React 18 + Vite + Tauri 2. No third-party UI lib — all hand-rolled CSS custom properties.

---

## 1. Design Tokens

### Background Hierarchy

| Token | Value | Usage |
|---|---|---|
| `--void` | `#121317` | Page background, deepest layer |
| `--panel` | `#1B1D23` | Sidebar, dialog body |
| `--raised` | `#242731` | Cards, hover states, elevated surfaces |
| `--overlay` | `rgba(10, 11, 13, 0.6)` | Dialog/modal scrim |

### Text

| Token | Value | Usage |
|---|---|---|
| `--text-primary` | `#F2F3F5` | Headings, body text |
| `--text-secondary` | `#9AA1AC` | Labels, secondary info |
| `--text-muted` | `#5E6470` | Placeholders, hints, disabled |
| `--text-on-accent` | `#FFFFFF` | Text on primary buttons |
| `--text-disabled` | `#4A4E58` | Disabled state text |

### Borders

| Token | Value | Usage |
|---|---|---|
| `--border-default` | `#2E323C` | Standard borders |
| `--border-subtle` | `#22252C` | Section dividers, subtle separation |
| `--border-focus` | `#4C8DFF` | Focus ring color |

### Signal Colors

| Token | Value | Semantic |
|---|---|---|
| `--signal-blue` | `#4C8DFF` | Primary, links, active |
| `--signal-blue-dim` | `#2A4A8A` | Active sidebar item bg |
| `--signal-green` | `#3FBE7A` | Success, enabled |
| `--signal-green-dim` | `#215C3B` | Success badge bg |
| `--signal-amber` | `#E0A339` | Warning |
| `--signal-amber-dim` | `#6B4E1A` | Warning badge/bg |
| `--signal-red` | `#E5484D` | Error, destructive |
| `--signal-red-dim` | `#6E2528` | Error badge bg |

### Semantic Colors

| Token | Map |
|---|---|
| `--color-primary` | `var(--signal-blue)` |
| `--color-primary-hover` | `#3E7AE0` |
| `--color-primary-text` | `var(--text-on-accent)` |
| `--color-enabled` | `var(--signal-green)` |
| `--color-disabled` | `var(--text-muted)` |
| `--color-warn` | `var(--signal-amber)` |
| `--color-warn-bg` | `var(--signal-amber-dim)` |
| `--color-block` | `var(--signal-red)` |
| `--color-block-bg` | `var(--signal-red-dim)` |
| `--color-destructive` | `var(--signal-red)` |
| `--color-link` | `var(--signal-blue)` |

### Typography

**Font stacks:**
- `--font-ui`: `'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif`
- `--font-mono`: `'JetBrains Mono', 'Cascadia Code', ui-monospace, monospace`

**Type scale (4 steps only):**

| Token | Size | Usage |
|---|---|---|
| `--text-xs` | `0.75rem` (12px) | Labels, badges, fine print |
| `--text-sm` | `0.875rem` (14px) | Buttons, body |
| `--text-base` | `0.875rem` (14px) | Default body |
| `--text-lg` | `1rem` (16px) | Headings, modals |
| `--text-xl` | `1.375rem` (22px) | Display |

**Line height:**
- `--leading-tight`: `1.25`
- `--leading-normal`: `1.45`
- `--leading-relaxed`: `1.6`

**Weights:** `400` normal, `500` medium, `600` semibold, `700` bold.

**Letter spacing:**
- `--tracking-tight`: `-0.01em` (UI text)
- `--tracking-label`: `0.02em` (uppercase labels)

### Spacing (4px base grid)

| Token | Rem | px |
|---|---|---|
| `--space-0` | — | 2px |
| `--space-1` | — | 4px |
| `--space-2` | — | 8px |
| `--space-3` | — | 12px |
| `--space-4` | — | 16px |
| `--space-5` | — | 24px |
| `--space-6` | — | 32px |
| `--space-7` | — | 48px |

### Radii

| Token | Value |
|---|---|
| `--radius-sm` | 6px |
| `--radius-md` | 10px |
| `--radius-lg` | 14px |
| `--radius-pill` | 999px |

### Elevation

| Token | Value |
|---|---|
| `--shadow-flat` | `none` |
| `--shadow-raised` | `0 1px 2px rgba(0, 0, 0, 0.4)` |
| `--shadow-modal` | `0 8px 24px rgba(0, 0, 0, 0.55)` |

### Motion

| Token | Value |
|---|---|
| `--motion-fast` | `120ms ease-out` |
| `--motion-default` | `180ms ease-out` |

### Focus Ring

```
--focus-ring: 0 0 0 2px var(--void), 0 0 0 4px var(--border-focus)
```
Applied via `*:focus-visible` — two-layer ring for contrast against dark bg.

---

## 2. Component Specifications

### Buttons (`.btn`)

Base: `display: inline-flex`, `gap: 8px`, `border-radius: 10px`, `min-height: 36px`.

| Variant | Class | bg / border | text | padding |
|---|---|---|---|---|
| Primary | `.btn-primary` | `--signal-blue`, none | `--text-on-accent` | `0 16px` |
| Secondary | `.btn-secondary` | transparent, `1px solid --border-default` | `--text-primary` | `0 16px` |
| Destructive | `.btn-destructive` | transparent, `1px solid --signal-red` | `--signal-red` | `0 16px` |
| Ghost | `.btn-ghost` | transparent, none | `--text-secondary` | `0 8px` |

Size modifiers:
- `.btn-sm`: `height: 32px`, `padding: 0 12px`, `font-size: --text-xs`
- `.btn-icon`: `width: 32px`, `height: 32px`, `padding: 0`
- `svg` inside: `16x16` (standard), `18x18` (icon-only)

States: `:hover` darkens/lights, `:active` scales `0.98`, `:disabled` → `opacity: 0.4`, `cursor: not-allowed`.

### Toggle Switch (`.toggle`)

- Track: `36x20px`, `border-radius: 999px`, `--raised` bg, `--border-default` border.
- Knob: `14x14px`, `--text-primary`, `box-shadow`.
- `:checked` → track `--signal-green`, knob `translateX(16px)`.
- `:disabled` → `opacity: 0.4`.

### Mod Card (`.mod-card`)

- Surface: `--raised` bg, `1px solid --border-subtle`, `--radius-lg` (14px).
- Padding: `--space-4` (16px) all sides.
- Gap between children: `--space-3` (12px).
- `:hover` → `--border-default` border.
- `.disabled` → `opacity: 0.55`.

Sub-elements:
- `.mod-priority-handle`: `24x40px`, `cursor: grab`, `--text-muted`.
- `.mod-priority-num`: `24x24px` badge, `--border-subtle` bg, mono font.
- `.mod-name`: `--text-sm`, `--weight-medium`, ellipsis overflow.
- `.mod-path`: mono font `12.5px`, `--text-secondary`.
- `.mod-source`: `--text-xs`, mono font, `--text-muted`.

### Dialog / Modal (`.dialog-overlay` + `.dialog`)

- Overlay: fixed fullscreen, `--overlay` bg, `fadeIn` 120ms.
- Dialog: `--panel` bg, `1px solid --border-default`, `--radius-lg`, `--shadow-modal`.
- Padding: `--space-5` (24px), `min-width: 400px`, `max-width: 520px`.
- Entry: `slideUp` 180ms.
- Footer: `flex-end`, `gap: 8px`.

### Input (`.input`)

- Size: `height: 36px`, `padding: 0 12px`.
- Border: `1px solid --border-default`, `--radius-md`.
- `:hover` → `--text-muted` border.
- `:focus` → `--border-focus` border + focus ring.
- `::placeholder` → `--text-muted`.
- `.input-mono`: mono font variant.
- `.input-label`: uppercase `--text-xs`, `--tracking-label`, `--text-secondary`.
- `.input-hint`: `--text-xs`, `--text-muted`.
- `.input-error`: `--text-xs`, `--signal-red`.
- Wrapper `.input-group`: `margin-bottom: 16px`.

### Toast (`.toast`)

- Container: fixed `bottom: 16px`, `right: 16px`, `z-index: 200`, column gap `8px`.
- Toast: `--raised` bg, `1px solid --border-default`, `--shadow-raised`, `--radius-md`.
- Padding: `12px 16px`, `min-width: 280px`, `max-width: 400px`.
- Entry: `slideInRight` 180ms.
- Variants (left border accent): `.toast-success` → `3px solid --signal-green`, `.toast-warning` → `--signal-amber`, `.toast-error` → `--signal-red`.

### Badges

**Status badges (`.badge-status`):**
- Inline flex, `padding: 1px 8px`, `--radius-sm`, `line-height: 18px`.
- `.conflict-warn`: `--signal-amber-dim` bg, `--signal-amber` text.
- `.conflict-block`: `--signal-red-dim` bg, `--signal-red` text.
- `.enabled`: `--signal-green-dim` bg, `--signal-green` text.

**Support badges (`.badge-support`):**
- `font-size: 10px`, uppercase, `letter-spacing: 0.02em`.
- `.verified`: green dim/amber.
- `.provisional`: amber dim/amber.

### Safety Note (`.safety-note`)

- Warning banner: `--signal-amber-dim` bg, `--signal-amber` text.
- Padding: `8px 12px`, `--radius-md`, `--text-xs`.
- Flex with `gap: 8px`, icon aligned top.

### File Path Display (`.path-display`)

- `--void` bg, `1px solid --border-default`, `--radius-md`.
- Mono font `12.5px`, `--text-secondary`.
- Text ellipsis overflow.

### Verification Checklist (`.checklist`)

- Column `gap: 8px`.
- Items: flex `gap: 8px`, `--text-sm`, `--text-secondary`.
- Icons tinted by state: `.pass` → `--signal-green`, `.fail` → `--signal-red`, `.pending` → `--text-muted`.

---

## 3. Layout

### App Shell

```css
grid-template-columns: 260px 1fr; /* sidebar | main */
height: 100%;
```

### Sidebar (`.sidebar`)

- `--panel` bg, `border-right: 1px solid --border-subtle`.
- `flex-direction: column`, `height: 100%`.
- Brand: flex row, `gap: 8px`, icon `28x28px`, `--signal-blue` bg.
- Section labels: uppercase, `--text-xs`, `--text-muted`.
- Game list: scrollable, `padding: 0 8px`.
- Game item: `height: 40px`, `padding: 0 12px`, `--radius-md`.
  - `:hover` → `--raised` bg, `--text-primary`.
  - `.active` → `--signal-blue-dim` bg, `--text-primary`.
- Footer: `padding: 12px 16px`, `border-top`.
- Add game button: dashed border, full width, `height: 36px`.

### Main Panel (`.main-panel`)

- `flex-direction: column`, `height: 100%`.
- `.panel-header`: `padding: 16px 24px`, `min-height: 56px`, `border-bottom`.
- `.toolbar`: `padding: 12px 24px`, `min-height: 48px`, `border-bottom`.
- `.mod-list`: scrollable, `padding: 12px 24px`.
- `.mod-list-empty`: centered column, `--text-muted`, `max-width: 280px`.

---

## 4. Animations

```css
@keyframes fadeIn        /* overlay entry */
@keyframes slideUp       /* dialog entry */
@keyframes slideInRight  /* toast entry */
```

All vars respect `prefers-reduced-motion` (durations → `0.01ms`).

---

## 5. Drag & Drop States

| State | Effect |
|---|---|
| `.mod-card.dragging` | `opacity: 0.5`, `border-color: --signal-blue` |
| `.mod-card.drag-over` | `--signal-blue` border + box-shadow glow |

---

## 6. Scrollbars

```css
width: 6px
track: transparent
thumb: --border-default, --radius-pill
thumb:hover: --text-muted
```

---

## 7. Page & View Inventory

> Single-page Tauri v2 app. No router — view switching via React state in `App.tsx`.

### App Shell

```
┌─────────────────────────────────────────────┐
│  Sidebar (260px)        │  Main Panel       │
│                          │                   │
│  ┌──────────────────┐   │  ┌─────────────┐  │
│  │  Skuld logo      │   │  │ Panel header│  │
│  ├──────────────────┤   │  ├─────────────┤  │
│  │  GAMES           │   │  │  Toolbar    │  │
│  │  ○ Witcher 3  ✓  │   │  ├─────────────┤  │
│  │  ○ SoD2       ⚡  │   │  │ Mod list    │  │
│  │                  │   │  │ or empty    │  │
│  ├──────────────────┤   │  │ state       │  │
│  │  [+ Add Game]    │   │  │             │  │
│  └──────────────────┘   │  └─────────────┘  │
│                          │                   │
└─────────────────────────────────────────────┘
```

### Views (2 total)

#### View A: Empty State — "Select a game"
- **Condition:** No game selected.
- **Content:** SVG icon + "Select a game" heading + helper text.
- **Interactions:** None — user must click game in sidebar.

#### View B: Game Mod Manager
- **Condition:** Game selected.
- **Content:** See component breakdown below.

### Component Breakdown

| Component | Renders in | Purpose |
|---|---|---|
| **Sidebar** | Left panel, always visible | Game list, support badges, Add Game button |
| **ModList** | Main panel | Header, toolbar, save scanner, mod cards |
| **ModCard** | ModList | Per-mod row: drag handle, name, path, toggle, delete |
| **SaveScanner** | ModList, collapsible | Save file browser with timestamps |
| **ToastContainer** | Overlay, always rendered | Notification stack (auto-dismiss 5s) |

### Dialogs (6 total)

All rendered as overlay modals over scrim. One at a time via `dialog.mode` state.

| Dialog | Trigger | Content |
|---|---|---|
| **AddGameDialog** | [+ Add Game] sidebar button | Form: game type (Witcher 3/SoD2), display name, install path browser |
| **EditGameDialog** | [Edit] in panel header | Form: install path, optional launch executable |
| **ImportModDialog** | [Import Mod] in toolbar | Archive picker (.zip/.7z/.rar), mod name, 7z warning |
| **DeleteModDialog** | Trash icon on ModCard | Confirm: "Delete [modName] from [gameName]?" |
| **RemoveGameDialog** | [Remove] in panel header | Confirm: "Remove [gameName] from manager?" |
| **BackupRestoreDialog** | [Backups] in toolbar | Backup list with timestamps, Backup Now / Restore actions |

### Navigation Flow

```
App.tsx
├── Sidebar (left, fixed)
│   └── click game → setSelectedGameId
│
├── Main Panel (right)
│   ├── no game → Empty state: "Select a game"
│   └── game selected → ModList
│       ├── Panel header (game name, path, Launch/Edit/Remove buttons)
│       ├── Toolbar (Mod count badge, Deploy All, Purge All, Import Mod, Backups)
│       ├── SaveScanner (collapsible)
│       ├── Safety notice banner
│       ├── ModCard × N (drag-reorderable for Witcher 3, toggle/delete per mod)
│       └── Empty mod state (if game has no mods)
│
├── Dialog (conditional, one at a time)
└── ToastContainer (persistent overlay)
```

### User Actions Map

| User Goal | Action | Result |
|---|---|---|
| Browse games | Launch app | Sidebar shows game list |
| Manage mods | Click game | Main panel shows mod list |
| Add game | Click [+ Add Game] | AddGameDialog opens |
| Deploy mods | Click [Deploy All] | All mods enabled |
| Purge mods | Click [Purge All] | All mods disabled |
| Import mod | Click [Import Mod] | ImportModDialog opens |
| Enable/disable mod | Toggle on ModCard | Mod state changes |
| Delete mod | Click trash icon | DeleteModDialog confirms |
| Browse saves | Expand SaveScanner | Save file list shown |
| Backup configs | Click [Backups] | BackupRestoreDialog opens |
