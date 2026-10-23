# GPUControl Pro — Design Documentation

> **Version**: 0.2.0
> **Last Updated**: 2026-07-17
> **Status**: Phase 1 Complete, UI Rebuild In Progress

---

## Table of Contents

1. [Design Philosophy](#design-philosophy)
2. [Visual Design System](#visual-design-system)
3. [Layout Architecture](#layout-architecture)
4. [Component Design](#component-design)
5. [Interaction Design](#interaction-design)
6. [Information Architecture](#information-architecture)
7. [Animation & Motion](#animation--motion)
8. [Overlay Design](#overlay-design)
9. [Design Decisions](#design-decisions)
10. [Future Design Work](#future-design-work)

---

## Design Philosophy

### Core Principles

- **Clarity Over Flash**: GPU monitoring data must be immediately readable. Visual hierarchy prioritizes real-time metrics over decorative elements.
- **Dark-First**: Built for dark environments (gaming rigs, dark rooms). All UI assumes a dark background with light text as the default and only mode.
- **Performance Transparency**: The UI should never feel heavier than the data it displays. Animations are meaningful, not ornamental.
- **Gaming Aesthetic**: Inspired by ASUS Armoury Crate and MSI Afterburner — angular, dense with information, RGB-adjacent accent colors, and a sense of precision machinery.

### Design Goals

| Goal | Rationale |
|------|-----------|
| Sub-100ms perceived latency | GPU metrics must feel real-time |
| Readable at a glance | Monitoring during gaming sessions |
| Consistent hardware metaphors | Fans, clocks, power — physical-world analogs |
| Scalable information density | 1 GPU or 8 GPUs, same visual clarity |

---

## Visual Design System

### Color Palette

The application uses a dark gaming aesthetic with electric blue as the primary accent.

```
Background:         #0A0A0F  (deepest base)
Surface:            #12121A  (cards, panels)
Surface Elevated:   #1A1A26  (hovered/selected states)
Surface Border:     #2A2A3A  (subtle separators)

Primary Accent:     #00B4FF  (electric cyan-blue)
Primary Dim:        #0088CC  (less intense)
Primary Glow:       #00B4FF40  (glow effect alpha)

Success:            #00E676  (green)
Warning:            #FFD600  (amber)
Danger:             #FF1744  (red)

Text Primary:       #F0F0FF  (headings, key values)
Text Secondary:     #8888AA  (labels, descriptions)
Text Muted:         #555577  (tertiary info)

GPU Temp Gradient:  #00E676 → #FFD600 → #FF1744  (safe → caution → critical)
```

### Typography

| Role | Family | Weight | Size | Case |
|------|--------|--------|------|------|
| Display / Hero | Inter | 700 (Bold) | 32px | Normal |
| Headings | Inter | 600 (Semibold) | 16–20px | Normal |
| Metric Values | JetBrains Mono | 600 (Semibold) | 24–40px | — |
| Metric Labels | Inter | 500 (Medium) | 11–12px | Uppercase |
| Body Text | Inter | 400 (Regular) | 13–14px | Normal |
| Small / Captions | Inter | 500 (Medium) | 10–11px | Uppercase |

### Spacing System

A 4px base grid is used throughout:

| Token | Value | Usage |
|-------|-------|-------|
| Space-1 | 4px | Inner padding in tight groups |
| Space-2 | 8px | Between related elements |
| Space-3 | 12px | Standard padding inside cards |
| Space-4 | 16px | Between grouped sections |
| Space-6 | 24px | Section margins |
| Space-8 | 32px | Page padding, major gaps |
| Space-12 | 48px | Layout gutters |

### Shape & Corner Radius

| Token | Value | Usage |
|-------|-------|-------|
| Radius-Sm | 4px | Buttons, small indicators |
| Radius-Md | 8px | Cards, panels, inputs |
| Radius-Lg | 12px | Modals, large containers |
| Radius-Full | 9999px | Badges, avatars, toggle handles |

### Shadows & Glows

```
Card Shadow:    0 2px 8px rgba(0, 0, 0, 0.4)
Elevated Card:  0 4px 16px rgba(0, 0, 0, 0.5)
Accent Glow:    0 0 12px rgba(0, 180, 255, 0.3)
Critical Glow:  0 0 12px rgba(255, 23, 68, 0.3)
```

---

## Layout Architecture

### Window Structure

```
┌──────────────────────────────────────────────────────┐
│ TitleBar (custom, 32px)                              │
│ ┌──────────┐ ┌──────────────────────────────────┬──┐ │
│ │ Logo/Brand │ │      Window Controls            │X │ │
│ └──────────┘ └──────────────────────────────────┴──┘ │
├──────┬───────────────────────────────────────────────┤
│      │                                                │
│ Nav  │              Main Content Area                  │
│ Side │                                                │
│ Bar  │  ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│ (48px│  │ Metric   │ │ Metric   │ │ Metric   │       │
│ wide)│  │ Card     │ │ Card     │ │ Card     │       │
│      │  └──────────┘ └──────────┘ └──────────┘       │
│ Icons│                                                │
│ only │  ┌──────────────────────────────────────┐      │
│      │  │         Graph / Chart Area            │      │
│      │  └──────────────────────────────────────┘      │
│      │                                                │
│      │  ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│      │  │ Control  │ │ Control  │ │ Profile  │       │
│      │  │ Panel    │ │ Panel    │ │ List     │       │
│      │  └──────────┘ └──────────┘ └──────────┘       │
│      │                                                │
└──────┴───────────────────────────────────────────────┘
```

- **TitleBar**: Custom 32px draggable title bar with window controls (minimize, maximize, close)
- **Sidebar**: 48px icon-only navigation sidebar (space-efficient, gaming-appropriate)
- **Content Area**: Responsive grid that adapts to window size — minimum 900px wide, no maximum
- **No scroll on root**: Individual panels scroll internally

### Page Layouts

| Page | Layout Pattern |
|------|---------------|
| Dashboard | 3-column metric grid + full-width chart + control row |
| Monitoring | 4-column mini-metric grid + tabbed chart panel |
| Tuning | Dual-panel: clock/fan curve left, real-time feedback right |
| Profiles | List-detail: profile list left, editor right |
| Settings | Single-column form with grouped sections |
| Overlay | Single compact bar (separate window, always-on-top) |

---

## Component Design

### Component Hierarchy

```
┌─────────────────────────────────────────────┐
│                  Pages                       │
│  Dashboard | Monitoring | Tuning | Profiles  │
│  Settings | Alerts | Overlay | Remote         │
├─────────────────────────────────────────────┤
│              Feature Components              │
│  MonitoringDashboard | FanControlPanel       │
│  ClockControlPanel | ProfileManager          │
│  AlertManager | OverlayConfig                │
├─────────────────────────────────────────────┤
│              Composite Components            │
│  MetricCard | GraphCard | ControlSlider      │
│  StatRow | GpuSelector | FanCurveEditor      │
│  DataTable | ColorGauge | MiniChart          │
├─────────────────────────────────────────────┤
│               Base Components                │
│  Button | Input | Select | Toggle | Slider   │
│  Card | Modal | Tooltip | Badge | Progress   │
│  Dropdown | SegmentedControl | Spinner       │
├─────────────────────────────────────────────┤
│              Primitive Components            │
│  Icon | Text | Divider | Spacer | Flex       │
│  Grid | Portal | VisuallyHidden              │
└─────────────────────────────────────────────┘
```

### Key Component Design Patterns

#### MetricCard

The most frequently rendered component. Designed for maximum readability.

```
┌───────────────────────┐
│ ○ GPU Temperature     │ ← icon + label (uppercase, muted)
│                       │
│     72°C              │ ← value (large mono, primary accent or color-coded)
│                       │
│  ━━━━━━━━●━━━━━━      │ ← mini progress/range bar
│  45°C  ▏68°C  ▏89°C   │ ← min/current/max annotation
└───────────────────────┘
```

- Color-coded value based on threshold (green → yellow → red)
- Optional sparkline mini-chart in the background
- Click to expand into detailed GraphCard

#### GraphCard

Handles all time-series visualization.

```
┌──────────────────────────────────────────┐
│ Temperature                   3m ▾ 15m ▾ │ ← header with time range selector
│ ┌──────────────────────────────────────┐ │
│ │                                      │ │
│ │    ╱╲     ╱╲     ╱╲                  │ │
│ │   ╱  ╲   ╱  ╲   ╱  ╲    ╱╲          │ │
│ │  ╱    ╲ ╱    ╲ ╱    ╲  ╱  ╲         │ │
│ │ ╱      ╲      ╱      ╲╱    ╲        │ │
│ │                                        │ │
│ └──────────────────────────────────────┘ │
│ Core: 1875MHz ▏ Mem: 5001MHz ▏ PWR: 185W│ ← legend row
└──────────────────────────────────────────┘
```

- Smooth cubic bezier interpolation
- Area fill with gradient (accent blue to transparent)
- GPU temperature color zone backgrounds (subtle)
- Multi-line overlay support (compare metrics)

#### ControlSlider

Used for fan speed, clock offset, power limit adjustments.

```
Fan Speed
65% ─────────────────●───────────────────── 100%
    0%                                    100%
    [Current: 65%]        [Apply] [Reset]
```

- Large track (8px height) for easy mouse targeting
- Snap-to-increment behavior configurable per control
- Real-time value preview while dragging
- Undo/Reset support

---

## Interaction Design

### States

Every interactive component defines these states:

| State | Description | Visual Cue |
|-------|-------------|------------|
| Default | Resting state | Normal colors |
| Hover | Mouse over | +2px brighten, cursor change |
| Active / Pressed | Mouse down | Scale 0.97, darker bg |
| Focus | Keyboard focus | 2px accent ring |
| Disabled | Not available | Opacity 0.4, no interaction |
| Loading | Async operation | Spinner overlay or skeleton |
| Error | Operation failed | Red border + error message |

### Feedback Patterns

| Action | Feedback Type | Timing |
|--------|--------------|--------|
| Toggle switch | Immediate visual flip + 20px slide animation | 150ms |
| Slider drag | Real-time value update (every frame) | 16ms |
| Button click | Scale pulse + color flash | 100ms |
| Profile apply | Loading spinner + success checkmark | Until done |
| Critical alert | Persistent modal + tray notification | Until dismissed |
| Data refresh | Value cross-fade (old → new) | 200ms |

### Keyboard Navigation

- Tab / Shift+Tab through all interactive elements
- Arrow keys for sliders, segmented controls
- Space/Enter for buttons, toggles
- Escape to close modals, dropdowns
- Ctrl+1–9 for sidebar navigation shortcuts

---

## Information Architecture

### Navigation Structure

```
Main Navigation (Sidebar)
├── Dashboard       (/)          — Overview of all GPUs
├── Monitoring      (/monitor)   — Detailed per-GPU telemetry
├── Tuning          (/tuning)    — Overclocking and fan control
├── Profiles        (/profiles)  — Save/load/apply GPU profiles
├── Alerts          (/alerts)    — Alert rules and history
├── Overlay         (/overlay)   — In-game overlay configuration
├── Remote          (/remote)    — Remote monitoring setup
├── Automation      (/automation) — Rule-based automation
├── Marketplace     (/market)    — Community profile sharing
├── Reports         (/reports)   — Generated reports
└── Settings        (/settings)  — Application preferences
```

### Data Hierarchy (Monitoring Page)

```
Per-GPU Dashboard
├── GPU Selector (tab bar or dropdown)
├── Metrics Overview
│   ├── Temperature (current + min/max + trend)
│   ├── Core Clock (current + boost info)
│   ├── Memory Clock (current)
│   ├── Fan Speed (RPM + percentage)
│   ├── Power Draw (current + TDP limit)
│   ├── Voltage (current)
│   ├── Core Utilization (%)
│   └── Memory Utilization (%)
├── Charts Section
│   ├── Temperature History
│   ├── Clock Speed History
│   ├── Fan Speed History
│   └── Power Draw History
└── Controls Section
    ├── Fan Curve Editor
    ├── Clock Offset Controls
    ├── Power Limit Slider
    └── Voltage Offset
```

---

## Animation & Motion

### Principles

- **Fast**: All UI animations complete in 100–300ms (exception: 500ms for page transitions)
- **Subtle**: No bounce, no exaggeration. Linear or ease-out timing.
- **Functional**: Motion communicates state changes — never decorative.

### Timing Scale

| Token | Duration | Usage |
|-------|----------|-------|
| Instant | 50ms | Micro-interactions, press states |
| Fast | 150ms | Toggles, checkboxes, small elements |
| Normal | 250ms | Modals, panels, value transitions |
| Slow | 400ms | Page transitions, loading reveals |

### Specific Animations

| Element | Animation | Duration | Easing |
|---------|-----------|----------|--------|
| Sidebar icon hover | Background color fill | 150ms | ease-out |
| Metric value update | Cross-fade numeric | 200ms | ease-out |
| Graph line update | Path transition | 300ms | ease-out |
| Modal open | Fade in + scale 0.95→1 | 250ms | ease-out |
| Modal close | Fade out + scale 1→0.95 | 150ms | ease-in |
| Slider handle | Position lerp | 50ms | linear |
| Page transition | Fade 0→1 | 200ms | ease-out |
| Notification slide | Slide in from right | 300ms | ease-out |
| Spinner | Continuous rotation | 800ms/loop | linear |

---

## Overlay Design

The in-game overlay is a separate window with a distinct, minimal design.

### Visual Constraints

- **No chrome**: No title bar, no window controls, no backgrounds
- **Semi-transparent**: Configurable 25–100% opacity
- **Fixed position**: Anchored to a corner of the screen
- **Always-on-top**: Rendered above fullscreen games
- **Click-through**: Input passthrough to the game underneath

### Overlay Layout

```
┌──────────────────────────────┐
│ GPU 0: RTX 4090              │ ← GPU label (small, semi-transparent)
│ Temp   72°C   ████████░░ 82% │
│ Core   2745MHz ██████░░░ 65% │ ← compact metric rows
│ Mem    10501MHz ██░░░░░░ 22% │
│ Fan    1800RPM ████████░ 78% │
│ Power  385W    ████████░ 92% │
└──────────────────────────────┘
```

- Font: JetBrains Mono (monospace for alignment)
- Bar graphs use block characters for a retro terminal look
- Configurable metrics per user preference
- Auto-hide after configurable inactivity timeout

---

## Design Decisions

### Why Custom CSS Over Tailwind?

The original prototype used Tailwind CSS, but the UI rebuild (Phase 1) migrated to handcrafted CSS files for:

1. **Pixel precision**: Tailwind's arbitrary value syntax added noise for the level of precision needed
2. **Bundle size**: Eliminating Tailwind's utility classes reduced CSS bundle size
3. **Design token control**: Direct CSS custom properties give finer control over the color system
4. **Performance**: No runtime JIT compilation, no purging concerns

### Why 48px Icon-Only Sidebar?

- Gaming applications benefit from maximum vertical space for data
- Icons with tooltips are more discoverable than text labels when users have 10+ navigation targets
- Matches the density pattern of Armoury Crate and similar tools

### Why Custom TitleBar?

- Allows a cohesive dark theme (no OS chrome mismatch)
- Enables future features like GPU status indicator in the title bar
- Consistent cross-platform appearance

### Why No Light Mode?

- The audience (GPU overclockers, gamers) operates in dark environments
- Monitoring data is most readable on dark backgrounds
- Reduces development and testing surface by 50%

---

## Future Design Work

### Phase 2 Priorities

- Responsive layout breakpoints for smaller windows
- Custom chart rendering (replace Recharts with Canvas-based rendering for performance)
- Drag-and-drop dashboard customization
- Per-GPU theme colors

### Phase 3 Priorities

- Animated fan curves (preview airflow visualization)
- 3D GPU model rendering with thermal heatmap overlay
- Real-time power flow diagram
- Dark mode refinements (already dark, but deeper contrast exploration)

---

*Last updated: 2026-07-17*
