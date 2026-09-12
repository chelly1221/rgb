---
name: "RGB Switch"
description: "A compact charcoal control surface for local Windows RGB commands."
colors:
  ground: "#111113"
  surface: "#1c1c20"
  line: "#303036"
  muted: "#a5a3ad"
  text: "#f3f1f3"
  pink: "#ffa4d3"
  vermilion: "#ff7c6d"
  orange: "#ffab66"
  yellow: "#ffdc88"
  focus: "#ffbe85"
  on-ink: "#301b22"
  off-surface: "#2c2c32"
  off-text: "#e5e3e9"
  off-hover: "#36353c"
  scan-surface: "#30262f"
  scan-text: "#ffd0da"
  scan-hover: "#42313d"
  field-surface: "#1e1b23"
  titlebar-surface: "#151517"
typography:
  headline:
    fontFamily: "\"Pretendard Variable\", sans-serif"
    fontSize: "27px"
    fontWeight: 650
    letterSpacing: "-0.04em"
  title:
    fontFamily: "\"Pretendard Variable\", sans-serif"
    fontSize: "15px"
    fontWeight: 600
    letterSpacing: "-0.025em"
  device-title:
    fontFamily: "\"Pretendard Variable\", sans-serif"
    fontSize: "13px"
    fontWeight: 600
    letterSpacing: "-0.02em"
  body:
    fontFamily: "\"Pretendard Variable\", sans-serif"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "\"Pretendard Variable\", sans-serif"
    fontSize: "10px"
    fontWeight: 650
    letterSpacing: "0.03em"
rounded:
  control: "5px"
  mark: "0px"
  field: "7px"
  segment-group: "8px"
  command: "10px"
  empty: "12px"
  master: "16px"
spacing:
  control-inset: "3px"
  label-gap: "6px"
  compact-gap: "9px"
  group-gap: "12px"
  row-gap: "15px"
  section-gap: "20px"
  card-inset: "22px"
  page-inset: "32px"
components:
  button-on:
    textColor: "{colors.on-ink}"
    rounded: "{rounded.command}"
  button-off:
    backgroundColor: "{colors.off-surface}"
    textColor: "{colors.off-text}"
    rounded: "{rounded.command}"
  button-off-hover:
    backgroundColor: "{colors.off-hover}"
  button-scan:
    backgroundColor: "{colors.scan-surface}"
    textColor: "{colors.scan-text}"
    rounded: "{rounded.field}"
    padding: "9px 17px"
  button-scan-hover:
    backgroundColor: "{colors.scan-hover}"
  button-text:
    textColor: "{colors.muted}"
    rounded: "{rounded.control}"
    padding: "8px 3px"
  button-text-hover:
    textColor: "{colors.text}"
  field-search:
    backgroundColor: "{colors.field-surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.field}"
    padding: "10px 12px"
    width: "100%"
  device-switch:
    rounded: "{rounded.segment-group}"
    padding: "3px"
  master-card:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.master}"
    padding: "22px"
  titlebar:
    backgroundColor: "{colors.titlebar-surface}"
    height: "44px"
---

# Design System: RGB Switch

## Overview

**Creative North Star: "A compact light switch panel"**

A compact light switch panel for local Windows RGB control. Near-black ground, charcoal surfaces and warm white type keep the interface quiet; a pink-to-vermilion-to-orange-to-yellow spectrum identifies the ON action and the small flame mark.

The Korean interface uses a short type hierarchy, aligned controls and thin dividers. The custom titlebar belongs to the same visual surface. Selected device segments describe the last successful command; they do not imply measured illumination.

**Key Characteristics:**

- Charcoal surfaces with restrained warm-spectrum accents.
- Compact typography with Korean fallback support.
- Flat device rows and explicit paired ON/OFF commands.
- Custom window chrome and visible keyboard focus.

## Colors

The palette places a warm spectrum against near-black and charcoal neutrals. Frontmatter values are normative; the names below explain their roles.

### Primary

- **Pink, Vermilion, Orange and Yellow:** the four stops of the warm spectrum. Pink also colors links and the text caret; orange marks an available connection. The spectrum is a CSS linear gradient at (110deg), with stops at (0%, 38%, 70%, 100%).
- **ON Ink:** dark text over the warm ON fill.
- **Focus:** warm orange outline for keyboard interaction.

### Neutral

- **Ground:** the application canvas.
- **Surface:** the master command container.
- **Line:** list and settings dividers.
- **Text / Muted:** primary information and secondary descriptions.
- **Titlebar Surface:** a subtly separate strip for custom window chrome.
- **OFF Surface / OFF Text / OFF Hover:** the solid secondary master command.
- **Scan Surface / Scan Text / Scan Hover:** the muted rose discovery action.
- **Field Surface:** search input fill.

**The Warm Command Rule.** Concentrate the spectrum in the ON action, selected ON segment, flame mark and thin light strip; use solid charcoal for OFF.

## Typography

**Display and Body Font:** Pretendard Variable, bundled locally for Korean and Latin text across the entire app, including the titlebar and controls. Generic sans-serif is the emergency fallback.

The hierarchy is compact and utilitarian. Medium weights and slightly tightened headings distinguish controls without oversized display type. There is no separate editorial display face.

### Hierarchy

- **Headline:** the page title; reduces to (24px) at the compact breakpoint and (23px) at the narrow breakpoint.
- **Title:** the master section heading. Device-section headings use a smaller (13px) treatment.
- **Device title:** device names, with long names allowed to wrap.
- **Body:** device metadata. Supporting copy varies by context: page description and input text use (12px), while help text uses a more open line height (1.9).
- **Label:** compact ON/OFF segments. Master command labels use (15px, 750), reducing to (13px) on narrow screens.
- **Window name:** app identity uses (12px, 650), with letter spacing (-0.02em).

## Layout

A single centered column has a maximum width (864px), with the default page padding specified by the page-inset token at the top and sides and a bottom inset (18px). The titlebar stays at the top of the viewport. The desktop client defaults to (760 × 760) and has a native minimum (520 × 560).

The master container groups two equal-width commands in a two-column grid. A compact support summary sits below them. Device entries are flat horizontal rows with a category icon, flexible text, segmented commands and a detail disclosure. Use the existing dividers to structure the list.

At viewport widths up to (620px), page padding becomes (27px 23px 16px), the master inset becomes (19px), and master ON/OFF suffixes hide. At widths up to (440px), page padding becomes (24px 16px 15px), the master inset becomes (17px), and device command groups wrap below their names. These narrower layouts also support browser previews; they do not change the native minimum size.

Long device names, diagnostics and errors wrap. Help paragraphs are capped at (74ch). Search appears only when the discovered list contains more than four devices.

## Elevation & Depth

The implementation uses no box shadows. Tonal surfaces and thin borders establish grouping: the canvas supports the master container, while device rows remain directly on the ground. Expanded details use a subtly tinted inset surface. The titlebar has a bottom border and stays above scrolling content at z-index (5).

**The Flat Rows Rule.** Separate devices with hairline dividers; reserve an enclosing surface for a meaningful group or message.

## Shapes

Use the rounded scale in the frontmatter: restrained corners on small controls, softer corners on the master container, and an inset frame around paired device commands. The flame mark is the user-selected transparent three-flame PNG, displayed at 24 × 24px without a container. Connection indicators are circular. Utility icons are simple SVG strokes; device icons are typically (24px) with stroke width (1.5).

## Components

### Buttons

Master commands are broad and centered, with a minimum height (64px), reducing to (57px) on narrow screens. ON uses the spectrum with dark text; OFF uses charcoal with a visible border. ON hover brightens the fill and reveals a light border; OFF hover lightens its surface and border.

The discovery button uses a muted rose fill. Text buttons remain transparent and brighten their text on hover. Default state transitions affect background color, text color and border color over (150ms). Disabled buttons reduce opacity, with separate opacity values for master commands and window controls.

Keyboard focus uses a solid outline (2px) with offset (3px). Segmented commands use an inward offset (-2px); window controls use (-4px).

### Inputs / Fields

The search field is full-width, lightly tinted charcoal, with a thin border and muted placeholder. Text uses the main text color and the caret uses pink. Keyboard focus follows the shared outline. No custom invalid input state is defined.

### Cards / Containers

The master command container uses the master radius and card inset from the frontmatter, with a thin charcoal border. Empty-state containers use the empty radius and centered content. Errors and demo notices use tinted surfaces, small rounded corners, text and borders together to distinguish their meaning.

### Device command segments

A compact inset frame holds two explicit ON/OFF buttons. Unknown command state leaves both neutral. The selected OFF segment uses a brighter neutral fill; selected ON uses the spectrum. Selection is also expressed with aria-pressed and adjacent last-command copy. Unsupported devices retain their rows but disable commands.

### Detail disclosure

A labeled chevron expands a tinted diagnostic block below the row. The chevron rotates when expanded. Details and device-specific failures wrap instead of overflowing.

### Custom titlebar

The sticky strip combines a small spectrum flame mark, app name, draggable area and three window controls. The controls have equal widths (45px), reducing to (39px) on narrow screens; their height is (43px). Hover uses a neutral fill, with a red close hover treatment. Browser previews disable native window controls. Closing is disabled while a device command is busy. These samples describe appearance; native behavior is implemented by Tauri.

### Feedback

Busy and result feedback occupy a reserved strip, with icon and text announced through a polite live region. Errors use an alert region. The busy refresh icon rotates over (1s, linear, infinite); reduced-motion preference disables animations and transitions.

## Do's and Don'ts

### Do:

- **Do** reuse the warm spectrum for the ON command, selected ON segment, flame mark and thin light strip.
- **Do** show command state in both text and the selected segment.
- **Do** keep unsupported controls visibly disabled while retaining readable device information.
- **Do** use labeled SVG controls, visible focus and the reduced-motion behavior.
- **Do** keep technical details in the existing disclosure pattern.

### Don't:

- **Don't** add gradient text or blurred glow.
- **Don't** turn each device row into another raised card.
- **Don't** present a sent command as a measured physical lighting state.
- **Don't** introduce decorative dashboards or marketing sections into this utility.



