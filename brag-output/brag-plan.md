# Brag Plan: Orbit

## What is this app?
Orbit is a radial application launcher for Windows: hold the middle mouse button anywhere — even over another app — and a glassy ring of your apps blooms around the cursor; move toward one, release, and it launches. Native Rust + Tauri, a real `WH_MOUSE_LL` mouse hook, zero focus steal.

## The angle
The cursor is the hero. The video is one gesture, shown as a reflex: **hold → flick → released.** No taskbar hunt, no alt-tab, no window — the launcher *is* the pointer. The brag: "the fastest way to open anything on Windows is a button you already own."

## Hook (first 2-3 seconds)
A near-empty dark desktop. One line slams in: **"One button. Every app."** The cursor dot compresses — a middle-click — and the glass radial menu blooms around it, icons popping in with the real app's staggered pop. The product appears *under the text, around the cursor*, in under a second.

## Key moments (the middle)
- The selection ring snapping between items with the blue glow (#4f9cff) and the label pill appearing under the chosen item — the actual overlay CSS, recreated faithfully.
- The release: ring vanishes instantly, chosen app bursts open. (The real app closes "with no stale-frame flash" — show that instantness.)
- The same gesture performed *on top of a full-screen app window* — proving the "even when another application has focus" claim visually.

## Outro / punchline
The Orbit brand mark (the conic-gradient ring) draws itself, wordmark lands: **Orbit. Hold. Flick. Release.** Beat before the logo: the ring itself is the punchline — it was the menu all along.

## User flow worth showing
Desktop (nothing focused) → hold middle button → ring blooms around cursor → flick toward an app (ring + glow + label follow) → release → app launches, ring vanishes instantly. Then the same flow over an occupied screen.

## Tone
- Preset: `polished`
- Creative direction: desktop power-tool film — the cursor is the hero
- Interpretation: few scenes, longer holds, soft crossfades; confident restrained motion that snaps to the beat grid; no jokes, no flash — the product's instantness is the drama.

## Format: landscape — 1920x1080
## Duration: 19.5s

## Visual identity (from the project)
- Background: `#0e1117` (app token `--bg-app`); scene "desktop" is this dark void with a faint radial vignette
- Glass menu: `radial-gradient(circle at 50% 36%, rgba(255,255,255,0.05), rgba(13,16,24,0.92) 68%)`, 1px `rgba(255,255,255,0.1)` border, big soft shadow
- Accent: `#4f9cff` (selection ring, cursor dot, glow `rgba(79,156,255,0.5)`)
- Text: `#e8ecf4`, dim `#8b94a7`, faint `#66708a`
- Brand ring colors (outro): `#4f9cff → #7c6bff → #3ecf8e → #ffb44f → #ff5f6d → #4f9cff`
- Display font: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif (same family for body; weight 700 for display)
- Strongest visual element: the radial glass menu — dark disc, ring of rounded-square icon tiles (`border-radius: 26%`, `rgba(255,255,255,0.07)` bg, 1px `rgba(255,255,255,0.14)` border), selected tile scales 1.16 with blue border/glow, pill label under the selected item (`rgba(9,12,19,0.88)` pill, 12.5px text)

App tiles: 8 items, real-feeling names (VS Code, Spotify, Discord, Chrome, Figma, Steam, Terminal, Notepad) with stylized monogram icons — drawn glyphs, not trademarked logo assets.

## Share copy (draft)
I replaced my taskbar with one mouse button. Orbit is a radial launcher for Windows — hold middle-click anywhere, flick, release, and the app is open.

## Audio direction
- Role: polished warm bed with motion-matched accents
- Music: `happy-beats-business-moves-vol-12-by-ende-dot-app.mp3` (109.96 BPM), from bundled assets
- Music treatment: start at 0s, moderate volume posture, gentle fade-out over the last ~1.5s ending on the outro
- Music cue guidance: preset read (`happy-beats-business-moves-vol-12-by-ende-dot-app.music-cues.json`, 109.96 BPM, beat grid ≈ every 0.55s). Strong cues to target: **8.74s** (Scene 2 release/burst), **13.11s** (Scene 3 launch), **17.47s** (Scene 4 Orbit wordmark), **18.56s** (tagline settle). Beat-grid windows: Scene 1 icon bloom lands on the ~2.19s beat cluster; selection snaps in Scene 2 on beats ~5.34–7.64 (every other beat — sequential *text* labels hold ≥0.8s each).
- Audio-reactive treatment: subtle — the glass disc's inner glow and the accent glow breathe with the music's low band; no waveform bars, no pumping text
- SFX posture: moderate, motion-matched, professional restraint — one soft click on button press, airy whoosh on ring bloom, gentle tick per selection-ring snap, a satisfying swell on release/launch, a clean glass ping on the outro wordmark
- Audio-coupled moments: the hold (click), the bloom (whoosh), each flick (tick), each release (swell), the wordmark (ping)
- Restraint rule: no SFX during text holds; nothing louder than the swell; no comedic stingers

## Storyboard

### Scene 1 — "Hold" — 3.8s
Dark desktop void (#0e117 vignette). Headline slams in at ~0.3s and holds: **"One button. Every app."** A lone cursor dot drifts to frame center. At ~1.6s the dot compresses (middle-click, click SFX) and the glass radial menu blooms around it: dark disc scales up, 8 rounded-square app tiles pop in with the real app's stagger (≤40ms apart, one bloom feel), dashed dead-zone ring faint inside. Beat target: bloom completes by the ~2.19–3.27 beat window.
Sequential/interaction: yes — simulated middle-click; 8 tiles pop in staggered as one bloom
Audio intent: curiosity → spark; the click is the ignition
Audio-coupled idea: click on press, airy whoosh on the bloom, subtle rise into Scene 2
Music: warm bed entering, building
Transition mood: soft crossfade → Scene 2

### Scene 2 — "Flick" — 5.5s
The radial menu fills the frame — faithful recreation (glass disc, tile ring, selection ring, cursor dot). The cursor moves toward the Chrome tile: the selection ring + blue glow snap onto it (tick SFX), the pill label "Chrome" appears beneath. A second flick to "Spotify" — ring snaps again (tick), label swaps. Headline top-left, small: **"Flick."** At **8.74s (strong cue)** the button releases: the ring vanishes instantly, a launch burst flares from the cursor.
Sequential/interaction: yes — two simulated flicks, each with its own selection-snap and label hold (~0.8s+ each)
Audio intent: playful precision — every snap feels mechanical and satisfying
Audio-coupled idea: tick per snap on the beat grid (every other beat), swell on the 8.74s release
Music: bed grooving, accents land on snaps
Transition mood: clean cut → Scene 3

### Scene 3 — "Anywhere" — 5.2s
A mock full-screen app window (a plausible code editor, neutral dark UI, no real product chrome) fills the frame — clearly "another app has focus." The cursor sits mid-window. Same gesture: dot compresses at ~10.4s (click), the ring blooms *on top of* the editor around the cursor, flick to "VS Code" (snap + label), and at **13.11s (strong cue)** release — the editor world dims, a new app window flies open from the cursor. Caption settles in: **"Anywhere. Even over another app."** (holds ≥2s)
Sequential/interaction: yes — simulated hold/bloom/flick/release over an occupied screen
Audio intent: proof, delivered calmly — the gesture works over anything
Audio-coupled idea: click + whoosh (bloom), tick (snap), swell (13.11s launch)
Music: bed continues, launch swell is the peak
Transition mood: soft crossfade → Scene 4

### Scene 4 — "The ring" — 5.0s
Clean #0e1117 frame. The Orbit brand mark draws itself: a conic-gradient ring (#4f9cff → #7c6bff → #3ecf8e → #ffb44f → #ff5f6d) sweeps in like the menu opening one last time. At **17.47s (strong cue)** the wordmark lands: **Orbit** (700 weight, #e8ecf4), and at 18.56s the tagline settles under it: **"Hold. Flick. Release."** Small dim footer: *a radial launcher for Windows*. Music fades out by ~19.2s.
Sequential/interaction: yes — ring draws, then wordmark, then tagline (each held well past read floor)
Audio intent: resolution — the last soft ping and a warm tail
Audio-coupled idea: glass ping on the wordmark at 17.47s; nothing else
Music: fading bed, ends clean
Transition mood: end — hold on the final frame

**Music mood for this video:** upbeat-polished (warm confident bed, beat-synced accents)
**Audio summary:** one continuous warm 110 BPM bed; click → whoosh → tick-tick → swell peaks at 8.74s and 13.11s, then a single glass ping and fade to land the outro — every accent is motion-matched, never louder than the gesture it accompanies.
