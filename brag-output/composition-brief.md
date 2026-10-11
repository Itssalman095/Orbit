# Hyperframes Composition Brief: Orbit

## Objective
Create a short launch-style brag video for Orbit, a radial application launcher for Windows.

## Output
- Composition directory: `brag-output/composition/`
- Rendered video: `brag-output/brag.mp4`
- Format: landscape — 1920x1080
- Duration: 19.5s (15-25s window)

## Source Material
- Project root: `C:\Users\furka\Downloads\salman\Projects\Orbit`
- Primary files read: `README.md`, `src/styles/overlay.css`, `src/styles/global.css`, `src/styles/settings.css`, `src/components/RadialMenu.tsx`, `src/components/RadialItem.tsx`, `src/components/SelectionIndicator.tsx`, `src/overlay/Overlay.tsx`
- Product name: Orbit
- Tagline / strongest claim: "Hold the middle mouse button anywhere in Windows, move toward an application, release to launch." — even when another application has focus.
- Key UI or visual moment to recreate: the radial overlay — a dark glass circular backdrop (`radial-gradient(circle at 50% 36%, rgba(255,255,255,0.05), rgba(13,16,24,0.92) 68%)`, 1px `rgba(255,255,255,0.1)` border, soft shadow), a ring of rounded-square icon tiles (`border-radius: 26%`, `rgba(255,255,255,0.07)` bg, 1px `rgba(255,255,255,0.14)` border), the selected tile scales to 1.16 with blue border `rgba(99,165,255,0.95)` + glow, a selection ring (2px blue circle) that snaps between tiles, a pill label under the selected item (`rgba(9,12,19,0.88)` pill, 12.5px, `#eef2fa` text, 1px `rgba(255,255,255,0.14)` border), a 7px blue cursor dot, and a faint dashed dead-zone ring in the center. Items pop in staggered (~40ms apart) like the real app's `radial-pop` animation (scale 0.35 → 1, cubic-bezier(0.2, 0.8, 0.2, 1)).
- Copy that must appear verbatim:
  - "One button. Every app." (hook)
  - "Flick." (scene 2 headline)
  - "Anywhere. Even over another app." (scene 3 caption)
  - "Orbit" (wordmark) + "Hold. Flick. Release." (tagline) + "a radial launcher for Windows" (footer)

## Creative Direction
- Tone preset: `polished`
- Creative direction: desktop power-tool film — the cursor is the hero
- Interpretation: 4 scenes, longer holds, soft crossfades, restrained confident motion snapped to the beat grid; no jokes, no flash — the product's instantness is the drama. Fast-in, then hold; every readable line holds past its read floor.
- Angle: The cursor is the hero. One gesture — hold → flick → released — shown as a reflex. The launcher *is* the pointer: no taskbar hunt, no alt-tab, no window.
- Hook: near-empty dark desktop, "One button. Every app." slams in, cursor dot compresses (middle-click), the glass radial menu blooms around the cursor with staggered icon pops.
- Outro / punchline: the Orbit brand ring (conic gradient) draws itself — it was the menu all along — wordmark "Orbit" lands, tagline "Hold. Flick. Release."
- Avoid:
  - Generic SaaS language ("streamline your workflow" etc.)
  - Abstract filler visuals (waveforms, particles, color washes)
  - Unrelated visual redesign — use the app's exact palette, radii, and copy

## Visual Identity
- Background: `#0e1117` (app token `--bg-app`), with a faint radial vignette suggesting a desktop
- Glass menu: `radial-gradient(circle at 50% 36%, rgba(255,255,255,0.05), rgba(13,16,24,0.92) 68%)`, border `1px solid rgba(255,255,255,0.1)`, shadow `0 18px 60px rgba(0,0,0,0.45)`
- Text: `#e8ecf4` (primary), `#8b94a7` (dim), `#66708a` (faint)
- Accent: `#4f9cff` (selection ring, cursor dot), glow `rgba(79,156,255,0.5)` / `rgba(79,156,255,0.2)`
- Brand ring colors (outro only): conic gradient `#4f9cff → #7c6bff → #3ecf8e → #ffb44f → #ff5f6d → #4f9cff`
- Display font: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif — weight 700 for display lines, 400/600 for labels
- Body font: same family
- Visual references from the project: the radial overlay (glass disc + tile ring + selection ring + label pill + cursor dot + dashed dead zone), the settings sidebar brand mark (conic-gradient ring with a dark center hole), the status-pill aesthetic. App tiles: 8 real-feeling names (VS Code, Spotify, Discord, Chrome, Figma, Steam, Terminal, Notepad) with stylized monogram glyphs — drawn letter tiles, NOT real trademarked logos.

## Storyboard
Use the storyboard in `brag-output/brag-plan.md` as the creative contract.

Scene summary:
1. "Hold" — 3.8s — dark desktop, "One button. Every app." holds; cursor middle-clicks at ~1.6s; glass ring blooms around cursor, 8 tiles pop staggered (≤40ms apart), dead-zone ring visible
2. "Flick" — 5.5s — the radial menu fills frame; cursor flicks to "Chrome" then "Spotify" (selection ring + glow snap, pill label appears each time, each label holds ≥0.8s); release at 8.74s strong cue — ring vanishes instantly with a launch burst
3. "Anywhere" — 5.2s — a mock code-editor window fills the frame (another app has focus); same gesture on top of it; flick to "VS Code"; release/launch at 13.11s strong cue; caption "Anywhere. Even over another app." holds ≥2s
4. "The ring" — 5.0s — clean `#0e1117`; conic-gradient brand ring draws like the menu opening; "Orbit" wordmark lands at 17.47s strong cue; "Hold. Flick. Release." settles at ~18.56s; dim footer "a radial launcher for Windows"; music fades out by ~19.2s

## Audio
- Audio role: polished warm bed with motion-matched accents (minimal but present — 2-3 subtle SFX families, nothing aggressive)
- Audio arc: one continuous warm ~110 BPM bed; ignition click → bloom whoosh → selection ticks → release swells (peaks at 8.74s and 13.11s) → single glass ping on the wordmark → fade out to land the outro
- Music: `happy-beats-business-moves-vol-12-by-ende-dot-app.mp3` (copied to `brag-output/composition/assets/music/`)
- Music treatment: start at 0s, volume ~0.35 (never above 0.5), gentle fade-out over the last ~1.5s ending clean by ~19.2s
- Music cue guidance: bundled preset at `C:\Users\furka\Downloads\salman\Projects\Orbit\.agents\skills\brag\assets\music\cues\happy-beats-business-moves-vol-12-by-ende-dot-app.music-cues.json` — 109.96 BPM, beat grid ≈ every 0.55s (0.56, 1.09, 1.64, 2.19, 2.73, 3.27, 3.82, 4.39, 4.91, 5.34, 6.00, 6.56, 7.09, 7.64, 8.19, 8.74, 9.29, 9.83, 10.37, 10.93, 11.46, 12.02, 12.55, 13.11, 13.64, 14.20, 14.73, 15.29, 15.84, 16.38, 16.93, 17.47, 18.02, 18.56, 19.10, 19.66). Strong-cue locks (use 1-3): **8.74s** (scene 2 release), **13.11s** (scene 3 launch), **17.47s** (Orbit wordmark). Sequential non-text accents (tile pops, ticks) may hit every beat; sequential TEXT (labels) hold ≥0.8s each — snap to every other beat, not every beat.
- Audio-reactive treatment: subtle — the glass disc's inner glow and accent glow breathe with the music's RMS/bass; maybe faint vignette warmth. No waveform bars, no pulsing text, no strobing.
- Audio-coupled moments:
  - Scene 1 press (~1.6s) — soft click (`ui/mouseclick1` or `interface/click_*`)
  - Scene 1 bloom (~1.7-2.2s) — airy whoosh / gentle reveal (`interface/drop_*` or soft `impact/impactSoft_medium_*`)
  - Scene 2 flick snaps (~5.3-7.6s) — selection tick (`interface/select_008` or `ui/rollover*`, soft)
  - Scene 2 release (8.74s) — satisfying swell (`impact/impactSoft_medium_*` or `casino/chips-collide-*`)
  - Scene 3 press+release (10.37s, 13.11s) — repeat the gesture's click + swell
  - Scene 4 wordmark (17.47s) — one clean glass ping (`interface/bong_001` at low volume, or `impact/impactGlass_light_002`)
- SFX selection guidance: motion-matched, polished restraint — 0.55-0.85 volume with softer values for this polished tone; align SFX to the START of each animation; thin out rather than stack
- SFX analysis guidance: `C:\Users\furka\Downloads\salman\Projects\Orbit\.agents\skills\brag\assets\sfx\sfx-analysis.md` — prefer low/medium HF-risk files for repeated or polished moments
- Exact SFX choice: Hyperframes should choose filenames, timestamps, density, and volume based on the implemented animation.
- Audio files: music is already copied to `brag-output/composition/assets/music/`; Hyperframes copies its chosen SFX into `brag-output/composition/assets/sfx/`

## Hyperframes Instructions
Load the composition-building Hyperframes domain skills — `hyperframes-core` (composition contract + `data-*` timing), `hyperframes-animation` (motion), `hyperframes-creative` (design spec, beats, audio-reactive), `hyperframes-keyframes` (seek-safe keyframes), and `hyperframes-cli` (lint/check/render). /brag is its own workflow: do not enter the `hyperframes` entry-point intent interview and do not route into its generic promo / launch-video workflow. Prefer native Hyperframes conventions over anything in `/brag`.

Requirements:
- Show at least one real UI, copy, or visual element from the source project — the radial menu recreation IS the centerpiece; make it faithful (exact radii feel, colors, stagger, pill labels).
- Keep all text readable in the final render.
- Keep the video within 15-25 seconds (target 19.5s).
- Include the planned music/SFX layer.
- Treat /brag audio notes as guidance, not a fixed cue sheet. Choose SFX after the visual animation exists.
- Treat music cue metadata as optional timing hints; ignore cues that hurt readability or pacing. 1-3 strong cue locks max.
- Consider the Hyperframes audio-reactive workflow for the subtle glow breathing.
- Run `hyperframes check` before render — it is brag's single gate.
- Keep creation and rendering local. No remote/publishing workflows.
