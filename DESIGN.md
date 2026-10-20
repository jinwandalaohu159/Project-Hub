---
version: alpha
name: project-hub-claude-dark
description: Claude-inspired warm dark theme for Project Hub — warm charcoal canvas, coral accent, serif display type, and layered surfaces that evoke editorial quality rather than cold IDE utility.

colors:
  primary: "#cc785c"
  on-primary: "#1a1412"
  canvas: "#1a1614"
  ink: "#e8e0d8"
  surface: "#23201e"
  surface-raised: "#2c2825"
  surface-overlay: "#353130"
  muted: "#8a827a"
  border: "#3a3632"
  border-subtle: "#2e2a27"
  success: "#7eb87a"
  warning: "#d4a856"
  error: "#d46a6a"
  info: "#6a9fd4"

typography:
  display:
    fontFamily: "'Cormorant Garamond', Georgia, serif"
    fontSize: 28
    fontWeight: 600
    lineHeight: 1.2
  h1:
    fontFamily: "'Cormorant Garamond', Georgia, serif"
    fontSize: 22
    fontWeight: 600
    lineHeight: 1.3
  h2:
    fontFamily: "'Cormorant Garamond', Georgia, serif"
    fontSize: 17
    fontWeight: 600
    lineHeight: 1.4
  h3:
    fontFamily: "'Inter', system-ui, sans-serif"
    fontSize: 13
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: 0.02em
    textTransform: uppercase
  body:
    fontFamily: "'Inter', system-ui, -apple-system, sans-serif"
    fontSize: 13
    fontWeight: 400
    lineHeight: 1.5
  body-sm:
    fontFamily: "'Inter', system-ui, -apple-system, sans-serif"
    fontSize: 12
    fontWeight: 400
    lineHeight: 1.5
  button:
    fontFamily: "'Inter', system-ui, -apple-system, sans-serif"
    fontSize: 13
    fontWeight: 500
    lineHeight: 1
    letterSpacing: 0.01em
  code:
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace"
    fontSize: 13
    fontWeight: 400
    lineHeight: 1.6
  code-sm:
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace"
    fontSize: 11
    fontWeight: 400
    lineHeight: 1.5

rounded:
  sm: 4
  md: 8
  lg: 12
  full: 9999

spacing:
  xs: 4
  sm: 8
  md: 12
  lg: 16
  xl: 24
  section: 32

components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "6px 14px"
    hover:
      backgroundColor: "#d9896e"
    active:
      backgroundColor: "#b86a50"
  button-ghost:
    backgroundColor: transparent
    textColor: "{colors.muted}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "6px 10px"
    hover:
      backgroundColor: "{colors.surface-raised}"
      textColor: "{colors.ink}"
  button-icon:
    backgroundColor: transparent
    textColor: "{colors.muted}"
    rounded: "{rounded.md}"
    padding: "6px"
    hover:
      backgroundColor: "{colors.surface-raised}"
      textColor: "{colors.ink}"
  panel:
    backgroundColor: "{colors.surface}"
    border: "1px solid {colors.border-subtle}"
    rounded: "{rounded.lg}"
  panel-header:
    typography: "{typography.h3}"
    textColor: "{colors.muted}"
    padding: "10px 16px"
    borderBottom: "1px solid {colors.border-subtle}"
  input:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.ink}"
    border: "1px solid {colors.border}"
    rounded: "{rounded.md}"
    padding: "6px 10px"
    typography: "{typography.body}"
    focus:
      borderColor: "{colors.primary}"
      boxShadow: "0 0 0 2px rgba(204, 120, 92, 0.2)"
  tree-item:
    padding: "3px 8px"
    rounded: "{rounded.sm}"
    hover:
      backgroundColor: "{colors.surface-raised}"
    active:
      backgroundColor: "{colors.surface-overlay}"
  card:
    backgroundColor: "{colors.surface}"
    border: "1px solid {colors.border-subtle}"
    rounded: "{rounded.lg}"
    padding: "{spacing.lg}"
  modal-overlay:
    backgroundColor: "rgba(10, 8, 6, 0.7)"
  modal:
    backgroundColor: "{colors.surface-raised}"
    border: "1px solid {colors.border}"
    rounded: "{rounded.lg}"
    padding: "{spacing.xl}"
---

## Overview

Project Hub uses a Claude-inspired warm dark theme. The canvas is a deep warm charcoal — not cold blue-black — giving the interface an editorial, lived-in quality. Coral (`#cc785c`) serves as the singular accent color, inherited from Claude's design DNA, used sparingly on primary actions and focus states.

Typography pairs Cormorant Garamond (serif display) with Inter (sans-serif body), echoing Claude's editorial voice — authoritative but approachable. Code uses JetBrains Mono for crisp readability.

Surfaces layer through warm gray differentiation rather than heavy shadows: canvas → surface → surface-raised → surface-overlay. Borders are subtle warm lines that define structure without drawing attention. The overall feeling should be: **a well-lit study, not a hacker terminal**.

Key signature patterns:
- Panel headers use uppercase h3 labels in muted tone
- File tree items highlight with warm surface fills on hover
- Primary buttons are coral with dark ink, never outlined
- Empty states use serif display type for character
- All transitions are 150ms ease — present but never theatrical

## Colors

The palette is built on a warm charcoal base with a single coral accent. No blue, no purple, no cold tones.

**Canvas (`#1a1614`)** is the deepest background — used for the app shell and code editor background.

**Surface (`#23201e`)** is the default panel and card background. One step lighter than canvas.

**Surface-raised (`#2c2825`)** is for hover states, dropdowns, and elements that need to lift off the surface.

**Surface-overlay (`#353130`)** is for modals, tooltips, and the highest elevation layer.

**Primary (`#cc785c`)** is coral — used only for primary actions, focus rings, and the rare highlight. Never use it for decoration or large fills.

**Ink (`#e8e0d8`)** is the main text color — warm off-white, not pure white. Pure white (`#fff`) is banned except for monospace code.

**Muted (`#8a827a`)** is for secondary text, labels, and disabled states. Warm gray, not cool gray.

**Border (`#3a3632`)** is the default border color. Border-subtle (`#2e2a27`) is for inner dividers within panels.

Status colors follow the same warm bias:
- Success: muted sage green, not neon
- Warning: warm gold, not harsh yellow
- Error: muted terracotta, not bright red
- Info: soft steel blue, not electric blue

## Typography

**Cormorant Garamond** is the display typeface for h1, h2, and empty-state headlines. It brings editorial gravitas and visual contrast against the sans-serif UI. Use it only for headings and decorative text — never for body, labels, or buttons.

**Inter** is the workhorse for all UI text: body, buttons, labels, inputs, and h3 panel headers. h3 panel headers use uppercase tracking to differentiate from body text without switching typeface.

**JetBrains Mono** is the code font for the editor, file type badges, and any inline code. Use `code-sm` (11px) for badges and labels, `code` (13px) for the editor.

Font loading: load Cormorant Garamond (weights 400, 600) and Inter (weights 400, 500, 600) from Google Fonts or self-host. JetBrains Mono should be loaded for the Monaco editor.

Size hierarchy:
- Display: 28px — splash/empty states only
- H1: 22px — modal titles, major sections
- H2: 17px — panel titles (when serif is appropriate)
- H3: 13px uppercase — panel headers, section labels
- Body: 13px — default UI text
- Body-sm: 12px — metadata, secondary info
- Code: 13px — editor and inline code
- Code-sm: 11px — file type badges

## Layout

The app uses a three-column layout: **Project List | File Tree + Panels | Code Editor**.

**Left column** (240px): project list with add button at top. Fixed width.

**Center column** (flex): file tree at top, collapsible panels (branches, git status, commit log) below. The center column fills available space.

**Right column** (flex): Monaco editor. Takes the remaining space.

Panel dividers: use 1px `border-subtle` between columns — no drag handles unless resizing is implemented.

Within panels, use the spacing scale consistently:
- Panel padding: 16px
- Item spacing: 4px (tight lists) to 8px (card content)
- Section gaps: 12px
- Panel header padding: 10px 16px

Empty states: center content vertically and horizontally within the panel. Use serif display type for the headline, muted body-sm for the subtitle. No illustrations — the typography is the visual.

## Elevation & Depth

There is no drop shadow in this system. Elevation is communicated entirely through surface color differentiation.

The layer order from bottom to top:
1. **Canvas** — app background, editor background
2. **Surface** — panels, cards, sidebar
3. **Surface-raised** — hover states, dropdown menus, floating elements
4. **Surface-overlay** — modals, tooltips, popovers

The only box-shadow in the system is the focus ring on inputs: `0 0 0 2px rgba(204, 120, 92, 0.2)`.

Modals use a `rgba(10, 8, 6, 0.7)` overlay to dim the canvas beneath.

## Shapes

**Border radius** follows a four-tier system:
- `sm` (4px): inline badges, small tags, code blocks
- `md` (8px): buttons, inputs, tree items, dropdowns
- `lg` (12px): panels, cards, modals
- `full` (9999px): pills, avatar circles, toggle switches

Never mix radius sizes within the same component group. All items in a list use the same radius.

Images and icons: Lucide icons are stroke-based and naturally fit the system. No fill variants unless for status indicators.

## Components

### Button Primary
Coral background with dark ink text. Rounded-md (8px). Padding 6px 14px. Hover lightens coral to `#d9896e`, active darkens to `#b86a50`. No outlines, no shadows. Used for: "Add Project", "Confirm", primary modal actions.

### Button Ghost
Transparent background with muted text. Rounded-md. Padding 6px 10px. Hover fills with surface-raised and upgrades text to ink. Used for: secondary actions, panel controls.

### Button Icon
Square aspect, transparent background, muted text, rounded-md, padding 6px. Hover same as ghost. Used for: close, collapse, settings gear — any single-icon action.

### Panel
Surface background with border-subtle border, rounded-lg. Contains a header bar and content area.

### Panel Header
Uppercase h3 typography in muted color. 10px 16px padding. Bottom border-subtle divider. May contain icon buttons on the right side (ghost or icon style).

### Input / Select
Canvas background with border border, rounded-md, body typography. Focus state adds primary border and coral focus ring. Select dropdowns should use custom styling matching the input — never native browser selects.

### Tree Item
Compact padding (3px 8px), rounded-sm. Text in body-sm. File type badges use code-sm in a rounded-sm pill with surface-raised background. Hover shows surface-raised background. Active/selected shows surface-overlay background.

### Card
Surface background, border-subtle border, rounded-lg, 16px padding. Used for project list items and settings sections.

### Modal
Surface-raised background, border border, rounded-lg, 24px padding. Overlay is canvas at 70% opacity. Title uses h1 serif. Actions at bottom use primary + ghost buttons.

### File Type Badge
code-sm font, surface-raised background, muted text, rounded-sm, 1px 5px padding. e.g., "TS", "JS", "VUE".

## Do's and Don'ts

**Do:**
- Use warm grays for all neutral tones
- Use coral only for primary actions and focus states
- Use serif type for headings and empty states
- Keep transitions to 150ms ease
- Differentiate surfaces by color, not shadow
- Use Lucide icons for all icon needs
- Use Inter for all UI text below heading level

**Don't:**
- Use pure white (`#fff`) for text — use ink (`#e8e0d8`) instead
- Use cold blue, purple, or electric colors anywhere
- Use drop shadows for elevation
- Use native browser selects or inputs without custom styling
- Use text characters as icons ("+", "x", ">")
- Use large fills of coral — it's an accent, not a surface
- Use more than two typeface families in the UI (serif + sans is the limit)
- Mix border-radius sizes within the same component group

## Responsive Behavior

This is a desktop Tauri application with a fixed window. Responsive behavior is limited to:

- **Minimum window width**: 800px. Below this, panels should collapse into tab-based navigation.
- **Center column**: flex-grow with a minimum of 200px.
- **Right column** (editor): flex-grow with a minimum of 300px.
- **Left column**: fixed at 240px, collapsible to icon-only (48px) mode.

If the window is too narrow, prefer collapsing the left panel to icon-only mode before reducing the center or right columns.

## Iteration Guide

To extend this design system:

1. **New colors**: Must be warm-biased. Any new accent should be tested against the coral primary — if it competes, don't add it.
2. **New components**: Follow the surface layering pattern. Start with surface background, border-subtle, rounded-lg. Add elevation by moving up the surface scale.
3. **New typography sizes**: Add to the existing scale — don't create one-off sizes. Each new size must have a named token.
4. **Light mode**: Would require inverting the canvas/surface scale and switching ink to a warm dark tone. The coral accent can remain. Cormorant Garamond and Inter carry over unchanged.
5. **Animations**: Keep under 200ms. Prefer opacity and background-color transitions over transforms.

## Known Gaps

- **Monaco editor theme**: Needs a custom theme matching this palette (warm dark, coral for current line highlight, warm grays for gutters). The current `vs-dark` must be replaced.
- **Toast/notification system**: Not yet designed. Should use surface-overlay with ink text, rounded-lg, auto-dismiss.
- **Loading states**: Skeleton screens or spinners not yet defined. Should use surface-raised pulse animation.
- **Scrollbars**: Custom scrollbar styling needed (thin, surface-raised track, muted thumb).
- **Dark/light toggle**: Architecture supports it but only dark is designed.
- **Focus visible**: Keyboard focus styles for all interactive elements not yet fully specified beyond inputs.
