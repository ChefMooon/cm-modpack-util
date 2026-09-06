# CM Modpack Util Style Guide

## 1. Design Direction

CM Modpack Util uses a **micro-design technical interface** style: a compact,
information-dense desktop tool that makes system state visible and actions
predictable. Its visual language borrows from 1980s and early 1990s computer
interfaces without pretending to be a retro game.

The interface should feel like a well-made operator console:

- Precise, structured, and easy to scan.
- Dense enough for modpack maintenance, with deliberate whitespace around
	important actions.
- Bright, slightly electric accents against a warm technical base.
- Boxy and tactile, with visible boundaries and clear state changes.
- Functional first: decoration must never compete with project data or update
	decisions.

Use this guide with [desktop-ui-standards.md](desktop-ui-standards.md). The
desktop standards define behavior and accessibility requirements; this guide
defines the visual system.

## 2. Visual Principles

### Make the system legible

Show the source and state of information. Distinguish local Packwiz evidence,
application-owned metadata, network results, pending work, and errors through
labels, structure, and status text. Never rely on color alone.

### Prefer modules over decoration

Use compact panels, ruled sections, toolbars, tables, and status strips. A panel
should group a real task or a related set of facts. Avoid decorative cards that
do not contain an actionable or scannable unit of information.

### Make actions feel deliberate

Primary actions should be visually prominent but not oversized. Destructive or
wide-reaching operations must look distinct, state their consequence, and ask
for confirmation when required.

### Show change over time

Update review is about comparison. Use old and new values, severity markers,
diff-like rows, timestamps, and progress states to make change easy to verify.

### Keep the retro influence disciplined

Pixel-inspired details are welcome in borders, labels, status indicators, and
small decorative marks. Do not use unreadable pixel fonts for body text, fake
CRT scanlines behind dense content, or nostalgia as a substitute for hierarchy.

## 3. Color System

Define these as CSS custom properties in the application theme. Components
should use semantic variables rather than hardcoded color values.

| Token | Value | Use |
| --- | --- | --- |
| `--color-ink` | `#17212b` | Primary text and dark rules |
| `--color-paper` | `#f3efe2` | Main application background |
| `--color-panel` | `#fffaf0` | Raised work surfaces |
| `--color-panel-muted` | `#e5e0d1` | Secondary surfaces and disabled backgrounds |
| `--color-line` | `#68757d` | Borders, dividers, and inactive controls |
| `--color-cyan` | `#00a6b2` | Links, focused controls, and active navigation |
| `--color-yellow` | `#f2c94c` | Attention, pinned items, and pending work |
| `--color-coral` | `#d85b4a` | Errors, destructive actions, and critical updates |
| `--color-green` | `#3d8b68` | Success and completed operations |
| `--color-violet` | `#7566a8` | Secondary emphasis and external metadata |

The values above are the light-theme reference palette. The token names are
semantic roles, not promises that the same hex value will be used in every
theme. Components must consume the roles so a theme can change without
rewriting component styles.

### Theme model

Support three user-facing modes:

- **System:** Follow the operating system's light or dark preference. This is
  the default.
- **Light:** Use the warm paper-and-panel palette regardless of the system.
- **Dark:** Use the ink-console palette regardless of the system.

Apply the selected mode at the document root, for example with
`data-theme="light"`, `data-theme="dark"`, or `data-theme="system"`. Persist the
user's explicit selection. When the selection is `system`, respond to operating
system preference changes while the application is open. Do not overwrite an
explicit light or dark selection when the system preference changes.

### Theme reference values

Dark mode should feel like a low-light operator console, not a second visual
identity. Preserve the same hierarchy, accents, terminology, and information
density while changing surface and text values.

| Token | Light | Dark | Use |
| --- | --- | --- | --- |
| `--color-ink` | `#17212b` | `#f3efe2` | Primary text and inverse rules |
| `--color-paper` | `#f3efe2` | `#10161b` | Main application background |
| `--color-panel` | `#fffaf0` | `#182229` | Raised work surfaces |
| `--color-panel-muted` | `#e5e0d1` | `#24333a` | Secondary surfaces and disabled backgrounds |
| `--color-line` | `#68757d` | `#60747b` | Borders, dividers, and inactive controls |
| `--color-cyan` | `#007f88` | `#42d1d6` | Links, focus, and active navigation |
| `--color-yellow` | `#8b6d00` | `#f2c94c` | Attention, pinned items, and pending work |
| `--color-coral` | `#a83f35` | `#ef8070` | Errors, destructive actions, and critical updates |
| `--color-green` | `#2e6f54` | `#72bd8f` | Success and completed operations |
| `--color-violet` | `#5f528d` | `#b4a7ed` | Secondary emphasis and external metadata |

Theme-specific status colors must be checked against their active surface.
Avoid lowering contrast merely to make an accent look more neon in dark mode.
Use a light text color on dark panels and a dark text color on light panels;
never rely on opacity alone to create disabled or secondary text.

### Theme transition

Theme changes should be immediate and should not move or resize content. A short
120-180ms transition may be used for background, border, and text color changes,
but do not animate the entire layout. Respect reduced-motion preferences by
removing that transition. Native controls, dialogs, menus, and tooltips must
use the active theme rather than retaining stale colors.

### Color rules

- Use `--color-paper` and `--color-panel` as the dominant surfaces in both
	themes. Light mode should feel warm and technical; dark mode should feel
	like a quiet low-light console rather than a generic dark dashboard.
- Use one accent per interaction or state. Do not make every panel colorful.
- Pair every state color with text, an icon, a pattern, or a position change.
- Keep text, border, and focus contrast strong on both paper and panel surfaces
	in both themes.
- Use coral sparingly. It should retain urgency.
- Do not use gradients for controls, panels, or status labels.
- Do not hardcode a light or dark surface color inside a component. Use semantic
	tokens so the component follows the selected theme.

## 4. Typography

Typography should combine a utilitarian monospace face for technical values
with a readable sans-serif face for labels and longer text.

Recommended font stack:

```css
:root {
	--font-ui: "IBM Plex Sans", "Segoe UI", sans-serif;
	--font-mono: "IBM Plex Mono", "Cascadia Mono", Consolas, monospace;
}
```

If the preferred font is not bundled or available, the fallback must remain
readable on Windows. Do not use a pixel font for paragraphs, forms, or tables.

| Role | Font | Guidance |
| --- | --- | --- |
| Page title | UI sans-serif | Bold, compact, sentence case |
| Section title | UI sans-serif | Medium or bold; identify the task or data group |
| Technical value | Monospace | Versions, paths, hashes, commands, IDs, timestamps |
| Status label | Monospace | Uppercase is allowed for short labels only |
| Body copy | UI sans-serif | Short, direct, and practical |
| Metadata | Monospace | Smaller than body copy, never lower than accessible minimum size |

Use a restrained type scale: 12px metadata, 14px body text, 16px controls, and
20-24px headings. Avoid oversized hero typography; this is a working desktop
tool.

## 5. Spacing, Borders, and Shape

Use a compact four-point spacing rhythm:

```css
:root {
	--space-1: 4px;
	--space-2: 8px;
	--space-3: 12px;
	--space-4: 16px;
	--space-5: 24px;
	--space-6: 32px;
}
```

- Default control height: 32px.
- Compact control height: 28px, only for dense tables and toolbars.
- Minimum icon button target: 32px square.
- Standard panel padding: 16px.
- Dense table cell padding: 8px 10px.
- Default corner radius: 2px.
- Use 0px radius for terminal-like blocks and hard-edged status strips.
- Avoid pill-shaped controls except for compact tags and non-interactive badges.

Borders are part of the hierarchy. Use a 1px solid border for normal panels and
controls. Use a 2px border for focus, selected states, or high-priority callouts.
Use inset rules and offset shadows sparingly to create a tactile, layered feel:

```css
.panel {
	border: 1px solid var(--color-line);
	box-shadow: 3px 3px 0 var(--color-ink);
}
```

Do not apply the offset shadow to every element. Reserve it for primary panels,
dialogs, and the most important action surfaces.

## 6. Layout and Information Density

Use a stable application shell:

1. A narrow project/navigation rail for context and switching.
2. A compact top bar for the current project, status, and global actions.
3. A main workspace with a clear page title, primary action, and task content.
4. An optional bottom status strip for scan state, Git state, or background work.

Within a page:

- Lead with the current task, not an ornamental dashboard summary.
- Keep related values in aligned rows or tables.
- Use a consistent left edge for headings, controls, and data.
- Keep primary actions visible while the main content scrolls where practical.
- Use fixed or minimum dimensions for tables, toolbars, counters, and stateful
	controls so updates do not shift the layout.
- On narrow windows, reduce columns and allow detail views to become stacked;
	never force horizontal clipping of critical actions or status text.

## 7. Component Language

### Navigation

Navigation items are compact and left-aligned. The active item uses a cyan rule,
stronger text, and a surface change. Do not indicate the active state with color
alone.

### Buttons

- Use short, verb-first labels: `Scan project`, `Review updates`, `Apply selected`.
- Primary buttons use cyan or ink emphasis and a clear hover/focus state.
- Secondary buttons use a panel surface with a visible border.
- Destructive buttons use coral only when the action is genuinely destructive.
- Icon-only buttons require an accessible name and a tooltip.
- Do not make every action a filled button. Links, menu items, and quiet buttons
	are preferable for low-priority actions.

### Tables and lists

Tables are a primary interface for mod inventory and update review.

- Use a strong header row with monospace column labels.
- Align versions, counts, dates, and hashes consistently.
- Keep mod names and action controls readable when the window is resized.
- Use row highlights for selection, not alternating saturated colors.
- Keep provider and side visible as text or labeled badges.
- Show unknown or malformed values explicitly as `Unknown`, never as an empty
	cell or a guessed value.

### Status indicators

Use a small label with an icon or text state, such as `READY`, `SCANNING`,
`CHANGES FOUND`, `BLOCKED`, or `FAILED`. Status labels may use uppercase
monospace text, but the surrounding explanation should remain normal sentence
case.

### Forms

Labels sit above controls in compact forms. Descriptions explain the effect or
tradeoff, especially for paths, network requests, update behavior, and cleanup.
Use visible validation messages close to the relevant field.

### Dialogs and confirmations

Dialogs use a hard boundary, compact title bar, and clear action grouping. State
what will change, which files or projects are affected, and what happens on
failure. Return focus to the initiating control after closing.

### Terminal and log surfaces

Use a dark ink surface with warm light monospace text for command output and
diagnostics. Keep terminal surfaces supplementary: users should not need to
decode logs to understand the result of a normal operation.

## 8. Motion and Feedback

Motion should communicate system activity, not add spectacle.

- Use a 120-180ms transition for hover, focus, and selection changes.
- Use a short stagger when a page first reveals a set of panels or rows.
- Use a restrained scan or pulse only for active background work.
- Show progress for scans, update checks, Packwiz operations, and changelog
	generation when duration is not immediate.
- Respect reduced-motion preferences by removing stagger, pulse, and movement.
- Never animate layout in a way that moves a control underneath the pointer.

Every asynchronous operation needs a visible pending, success, partial success,
or failure state. Toasts may supplement the state but must not be the only record
of an important result.

## 9. Copy and Tone

Copy is concise, technical, and calm. It should help the user decide what to do
next.

- Prefer `Packwiz files are ready to scan` over `Everything looks great!`.
- Prefer `3 updates require review` over `Whoa, lots of updates!`.
- Name the source of evidence: `Read from local TOML`, `Fetched from Modrinth`,
	or `Stored by CM Modpack Util`.
- Explain unknown, blocked, partial, and failed states directly.
- Use sentence case for most visible text. Reserve uppercase for short status
	labels and technical readouts.
- Do not use retro slang, fake command prompts, or jokes in error messages.

## 10. Accessibility Baseline

The retro visual direction must never reduce usability.

- Maintain readable contrast for text, controls, borders, and focus indicators.
- Provide a visible keyboard focus state with a 2px cyan or yellow outline.
- Ensure all controls have an accessible name and a logical tab order.
- Never communicate provider, side, severity, or operation state through color
	alone.
- Keep text resizable without clipping or overlapping controls.
- Support reduced motion and keyboard operation.
- Preserve meaning when color, decorative borders, or monospace styling are
	removed.

## 11. Implementation Checklist

Before accepting a screen or component, verify:

- Does it look like a compact technical tool rather than a marketing page?
- Is the current project, task, and state obvious within a few seconds?
- Are local evidence, external data, and application metadata distinguishable?
- Does each important state have text or another non-color cue?
- Are borders, spacing, and control heights consistent with this guide?
- Can the complete workflow be performed with the keyboard?
- Does the layout remain stable while scanning, loading, filtering, or updating?
- Does the screen remain usable at the minimum supported desktop window size?
- Does reduced motion preserve the same information and feedback?

## 12. Reference Mood

Think: an early network utility rebuilt by a careful modern desktop team.
Technical labels, crisp rules, warm off-white surfaces, cyan terminals, and
compact status readouts should make the application feel distinctive. The user
should always come away with more certainty about their modpack than they had
before opening the tool.
