# Product

## Register

product

## Users

Field network technicians. Someone has plugged a laptop into a switch port at a customer site, an office wiring closet, or a colocation rack, and they need to know — in under 30 seconds — what switch they're connected to, which port it is, and which VLAN it's in. Then they close the app and move on.

The user is often:

- **Under time pressure.** They're interrupting a deployment, troubleshooting an outage, or labeling cables they shouldn't have to label.
- **In bad conditions.** Dim wiring closets, fluorescent glare, sometimes outdoors at the edge of a parking-lot deployment.
- **Mid-context-switch.** PortFinder is one of fifteen tools open. They want a single answer, not a session.

Secondary audiences exist (homelab users with a MikroTik, sysadmins doing inventory at a desk), but design choices serve the field tech first. If a decision helps the homelab user but slows the field tech, it loses.

## Product Purpose

PortFinder captures one CDP, LLDP, or MNDP packet from the network the host is connected to, parses the discovery TLVs, and shows the seven facts an admin actually wants: switch name, switch IP, switch port, native VLAN, voice VLAN, MTU, and switch model.

It runs as a desktop app (Tauri 2 + Rust + Svelte 5) on macOS, Windows, and Linux, with a CLI mode in the same binary for scripting. The app does one thing. The user opens it, picks an interface, hits Start, gets the answer, closes the app.

Success is the field tech reading the answer off the screen at a glance, with confidence that the values are right, and not having to think about the tool itself.

## Brand Personality

**Polished, branded, distinctive.** PortFinder is in the Linear / Raycast / Tailscale lane: a small utility that earns its dock icon by feeling considered and recognizable, not by mimicking the OS or by chasing SaaS chrome.

- **Voice**: plain, direct, slightly dry. No marketing voice, no exclamation points, no "let's get you set up!" enthusiasm.
- **Tone under failure**: honest. When a capture times out, when a switch doesn't speak the protocol, when MTU isn't in the packet — say so plainly. Don't paper over absence with hopeful empty states.
- **Identity**: a sibling to Baudrun (the same author's serial-terminal app). The two are recognizably from the same shop without being visually identical. Navy gradient + metallic connector aesthetic for the icon; the in-app surface should carry the same restraint and craft.

## Anti-references

The single explicit anti is **generic Electron app**: same Bootstrap-ish components as fifty other cross-platform utilities, every widget rounded the same way, no rooting in any visual tradition. PortFinder is a Tauri app, not Electron, but the *aesthetic* of "shipped fast, looks like everything else" is the trap.

By extension:

- Not a flashy SaaS dashboard (gradients, hero metrics, navy-and-neon).
- Not enterprise IT bloat (dense ribbons, dated chrome, every pixel claimed by a panel).
- Not AI-startup minimalism (cream background, 80% whitespace, useful info hidden to feel calm).

## Design Principles

These are strategic — they shape decisions, they don't dictate pixels. Visual rules live in DESIGN.md.

1. **The 30-second answer wins.** Every screen, every state, every word is judged against: does the field tech get switch + port + VLAN faster because of this, or slower? If neutral, default to less.
2. **Distinctive over conventional.** Don't ape the OS to feel safe. Don't ape SaaS to feel modern. PortFinder should be recognizable in a screenshot before any text loads. The user picked the personality lane; this principle is the consequence.
3. **Honest about absence.** When a value isn't in the packet, when capture failed, when the protocol isn't supported on the wire — say so directly with a real reason. No shrugs ("—"), no faux-empty cards, no false reassurance.
4. **Color is not the signal.** ~8–10% of the IT/engineering audience has color-vision deficiency. Every state (capturing, stopped, error, success, privilege warning) carries a text or icon cue alongside any color, never color alone. Treat this as a hard constraint, not a polish item.
5. **Motion serves comprehension or is absent.** Animation only when it makes a state change clearer (the capture progress bar). Everything else: no decorative motion. Respect `prefers-reduced-motion` as a first-class fallback, not an afterthought.

## Accessibility & Inclusion

- **WCAG AA** as a baseline; AAA on body text where it doesn't fight the chosen visual identity.
- **Color-blind safe** — explicit constraint. State and severity must be readable in greyscale or under deuteranopia/protanopia simulation.
- **`prefers-reduced-motion`** — the capture progress bar and any future animation must have a calm fallback (static fill, opacity transition, or nothing).
- **Keyboard reachable** — every action (Start, Stop, refresh, BPF install, language picker) is reachable and operable from the keyboard with visible focus styling. Not asked for explicitly in the interview but baseline-expected for a tool used by power users.
