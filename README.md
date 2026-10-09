# Wunderkammer WK-1

*A cabinet of improbable sound objects.*

Two cabinets, each holding one of twelve curious objects. Hit a cabinet and
you don't get a drum. You get a ping-pong ball accelerating to a roll, a
coin spinning down to a finite-time singularity, a door hinge, a tiny mouth
muttering a nonsense word, a bottle filling with water, a startled flock, a
music box whose pins were punched by the Collatz conjecture, a laser chirp off
a frozen lake, two black holes merging, a cellular automaton playing the
harmonic series, a pitch that falls forever, or a balloon let go across the room.

It shares its technology, its two-channel/MUTATE/plate-reverb layout and
much of its plumbing with [Xenokussion XK-1](https://github.com/mkforsb/xenokussion-xk1-slop):
Rust, a [Dioxus](https://dioxuslabs.com) UI, one codebase for two targets.

| Target | Audio | How |
| --- | --- | --- |
| Web (wasm) | WebAudio `AudioWorklet` | `make serve-web` |
| Native Linux | PulseAudio (also PipeWire's pulse server) | `make run-desktop` |

## Building

Requirements:

- Rust (stable) with `rustup target add wasm32-unknown-unknown`
- Dioxus CLI 0.7: `cargo binstall dioxus-cli` or grab `dx` from the Dioxus releases
- Desktop only (Debian/Ubuntu package names): `libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libpulse-dev libssl-dev pkg-config`

```sh
make serve-web      # http://127.0.0.1:8080, click once to open the cabinet
make run-desktop    # native window
make web desktop    # release bundles in target/dx/wunderkammer/release/
make test lint
make renders        # offline-render every preset to ./renders/*.wav
make spectrograms   # … and draw them (python3 + numpy + matplotlib)
make bench          # speed as a real-time factor
make calibrate      # loudness survey of every object (OBJECT=NAME for detail)
```

For the desktop build, `WUNDERKAMMER_LATENCY_MS` (default 20) sets the
PulseAudio target latency. To audition settings without the UI:

```sh
cargo run --release -p wunderkammer-dsp --example hit -- out.wav \
    engine=BALLOON pitch=0.6 x=0.75 y=0.35 z=0.85 time=0.75 seconds=3
```

## Playing it

- **Pads** (DISTURB): where you click sets the velocity, top is hard.
  Keys: `Z X C V` hit cabinet I softly to hard, `N M , .` cabinet II,
  `Space` both.
- **Knobs/faders**: drag vertically, `Shift` for fine, scroll to nudge,
  double-click to reset.
- **RND** rolls a new patch for the cabinet. **MUTATE** does it on every hit,
  part of the way (`clamp(current + (random − current) × MUTATE)`); at RND
  every hit is a brand new curiosity. The randomizer knows where each object
  sounds best (pitch and time ranges per object) and keeps the effects,
  replies and media usually off and occasionally wild. A test renders 600
  random patches and checks every one is audible and bounded.
- **Locks**: **Ctrl-click** (or ⌘-click) any control that RND and MUTATE
  would change (knobs, faders, the OBJECT grid, switches) to lock it. Locked
  controls turn sealing-wax red with a padlock and keep their value through
  RND and MUTATE until you Ctrl-click them again. Every control still draws
  its random number when locked, so locking one never changes what the
  others roll. The 🔒 button in the cabinet header shows the lock count and
  unlocks them all. OUTPUT, SENSE and PAN are never randomized, so they
  need no lock. Locks are saved with the panel.
- **Curiosities** (48 presets, grouped by object), **RND** and **→ I/II**
  (copy a cabinet to the other) are in each cabinet's header. Everything is
  saved automatically: to `localStorage` on the web and to
  `~/.config/wunderkammer/state.txt` on the desktop.
- The **ONE HIT** display is an offline render of one hit of the current
  patch, redrawn as you turn knobs.

## The objects

There is no shared envelope or exciter. Each object is a small simulation
that decides for itself how a hit unfolds, and hitting a busy object usually
*adds* to what it is doing (another ball, another puff of air) rather than
restarting it. PITCH and TIME mean what makes sense for each object (TIME is
how long the pour, spin, flight or inspiral lasts).

| Object | What it simulates | X | Y | Z |
| --- | --- | --- | --- | --- |
| **BOUNCE** | Balls on a struck drumhead under real ballistics (g = 9.81 m/s², coefficient of restitution). Each impact rings eight circular-membrane modes and the ball itself. The head can be driven like a vibrating table: the bouncing-ball-on-a-plate route to chaos, with phase locking, period doubling and Fermi acceleration | COUNT: 1 heavy ball … 24 grains (popcorn, snare sizzle) | BOUNCE: restitution | SHAKE: peak table acceleration Γ; above 1 g the table throws resting balls |
| **COIN** | Euler's disk: the rolling contact's wobble rate diverges as `(t₀ − t)^-p`, rapping the object's modes (and a wooden table) once per wobble, then the final clack | WOBBLE: the exponent; up makes the singularity sharper | RIM: smooth whirr … rattling bumps and grit | OBJECT: coin → pot lid → plate → singing bowl |
| **CREAK** | Stick-slip friction (Bilbao's soft bow law plus a Coulomb floor), solved implicitly per sample with Newton's method against the body's modal admittance and a soft hinge spring. A hand gesture sets speed and pressure over TIME, and each hit drags the other way | PRESSURE | GESTURE: swell → swipe → tremble → judder | BODY: door → balloon → wet glass → cello |
| **GLOSSA** | A Kelly–Lochbaum vocal tract (after Pink Trombone) shaped by a 3-parameter articulatory model (tongue place, constriction, lip opening). Each hit draws a nonsense word of 1–3 syllables (closures, glides, diphthongs) with an intonation: statement, question, surprise or sing-song | WORD: 48 fixed words, RND for a new one per hit | SOURCE: mouth pop per syllable (a talking drum) → voice → jaw harp → whisper | MOUTH: tract length 7 cm … 34 cm |
| **POUR** | A quarter-wave air column whose length shrinks as the bottle fills, so the pitch rises along `f₀/(1 − k·t)`, slowly then racing to the brim. It is excited by stream turbulence and by Minnaert bubbles that chirp upward as they rise (van den Doel's model) | FILL: up fills (up to +3.2 oct), down drains | FLOW: smooth stream → bubbly → glugging | FIZZ: carbonation crackle that outlasts the pour |
| **AVIARY** | A flock of seeded creatures: songbirds (whistles, sweeps, hooks, chips, trills), crickets (pulsed pure carriers) and frogs (buzzy harmonic croaks), each with a call grammar. The hit startles them all, then they calm down over TIME | SPECIES: 48 fixed, RND | FLOCK: 1–12 creatures, mixing species | FRENZY: rate, wilder sweeps, faster trills |
| **MUSICBOX** | Steel tines (cantilever modes 1 : 6.27 : 17.55) plucked by a cylinder whose spring runs down. The pins are number sequences: Collatz hailstones, Recamán, digits of π, Fibonacci mod m, Nørgård's infinity series. In the HARMONIC scale value `v` plays harmonic `v` folded into two octaves, so a Collatz path plays in pure just intonation and always cadences 8, 4, 2, 1 onto the tonic | TUNE: sequence and start, RND | SCALE: harmonic, pentatonic, major, hijaz, whole-tone, Bohlen–Pierce | WEAR: detune, missing pins, flutter, a buzzing comb |
| **ICE** | Flexural waves in a frozen lake: the plate's exact impulse response `t^-½·cos(x²/4at)`, whose frequency falls as 1/t². That gives the "pew" of thin ice and the Star Wars guy-wire blaster. Echoes come from further away, so they arrive later and slower | DISTANCE: 8 ms tick … 480 ms "peeeyoooo" | ECHOES: 1–12 paths | SHATTER: spontaneous cracks, each its own chirp |
| **INSPIRAL** | A compact binary: the gravitational-wave chirp `f ∝ τ^-3/8`, amplitude `∝ f^2/3`, on Keplerian orbits that circularise as `e ∝ f^-19/18`, then merger, a ringing quasi-normal mode and a thump of GW memory. It builds *towards* the climax, TIME after you hit | MASS: ratio 1–12 (odd harmonics) | ECCENTRIC: bursts at each periastron, accelerating | SPIN: precession wobble, longer and higher ringdown |
| **AUTOMATA** | 32 cells of an elementary cellular automaton; each live cell sounds one partial. Rule 90 draws a Sierpiński triangle in the spectrum. Past ~30 generations/s the chord changes turn into timbre | RULE: 16 picked rules, RND: a random lively rule and row | RATE: 2–300 gen/s | MAP: harmonic → odd → bell → undertone → pentatonic |
| **PARADOX** | A Shepard–Risset glissando: partials gliding under a fixed spectral window, falling forever. With PULSE, a Risset rhythm of five phase-aligned layers at doubling tempos makes the beat accelerate forever too | GLIDE: endless fall … hover … endless rise | PULSE: Risset rhythm depth | SPACING: a fifth … octave … tritave (through the golden ratio) |
| **BALLOON** | A neo-Hookean balloon: pressure `∝ 1/λ − 1/λ⁷` rises as it shrinks, peaks at λ = 7^⅙, then collapses ("wheeeEEEE-brrp"). Air leaves at ∝ √P. The nozzle is a pressure-driven Van der Pol flap with a second coupled lip mode. Let go, it jets around the room on a thrust-steered random path (with exaggerated Doppler, distance and spin) and lands with a plop | SIZE: initial stretch | FLAP: squeal → buzz → chaotic raspberry | FLIGHT |

## The rest of the cabinet

```
 PITCH ─┬─ BEND ─┬─ WHIM ─┬─ reply offset ─► pitch, X, Y, Z ─┐
        │        │        │                                 ▼
 DISTURB ─────────────────────────────────────────► OBJECT (1 of 12) ─► SHIFT ─► TONE ─► THROUGH ─► OUTPUT ─┬──────────► PAN ─► mix
                                                                                                           └─► PLATE ─┘
```

- **BEND**: a pitch sweep per hit (TIME, bipolar DEPTH) and **WHIM**:
  per-hit random offsets of pitch and the three macros.
- **SHIFT**: a single-sideband frequency shifter (a Hilbert pair from
  Olli Niemitalo's allpass coefficients) inside a feedback delay. Each trip
  round the loop shifts by the same number of Hz, so echoes spiral up or
  down like a barber pole. Small shifts beat and phase, large ones turn
  harmonic sounds into bent metal.
- **TONE**: a zero-delay-feedback state-variable filter (LP/BP/HP).
- **HEARD THROUGH**: where you hear the object from. AIR, a TIN CAN telephone
  (band-limited, metallic ring, a twanging string), a cardboard TUBE,
  underWATER (muffled, wobbling paths), through a WALL (muffled, a room
  boom and a slap from the next room), an AM RADIO (band-limited, driven,
  with static and a heterodyne whistle that only come with the signal) or a
  gramophone HORN (horn resonances, wow and dust).
- **REPLY**: this cabinet answers the *other* cabinet's hits. ODDS is the
  chance, LAG the delay (with a little human hesitation), and MOOD sets
  whether the conversation calms down (each reply quieter and lower) or
  gets riled (louder and higher). Replies can be answered in turn, so two
  cabinets set to reply hold a conversation, or an argument, that ends
  when it dies down or after 16 exchanges. It runs in the DSP, sample-accurate.
- **REVERB**: per cabinet, the same Dattorro plate as the XK-1 (AMOUNT, DECAY,
  TONE, PRE). PAN balances the stereo wet signal.
- **Engine and oversampling**: voices run at 2× with control updates every
  16 samples, decimated by a 47-tap windowed sinc, and each object has an
  output trim from `make calibrate`. With both cabinets hitting a heavy
  corner of every object through the full chain, the slowest (AVIARY with
  twelve birds) renders about 10× faster than real time on one desktop core
  and the fastest about 25× (see `make bench`). In the browser's wasm
  engine a single hit of any object renders 18–33× real time.

## Layout

```
crates/dsp        engine, the 12 objects, effects, plate reverb, parameters, presets and the
                  hit preview; no dependencies, 61 unit tests
  src/engines/      bounce, coin, creak, glossa, pour, aviary, musicbox, ice, inspiral,
                    automata, paradox, balloon
  src/fx.rs         SHIFT (frequency-shifter spiral) and THROUGH (the media)
  examples/         hit (settings → WAV), render (presets → WAV), bench, calibrate
crates/worklet    C ABI over the engine, compiled to a standalone wasm module with no imports
crates/app        Dioxus UI with `web` and `desktop` features, locks, RND/MUTATE, persistence
tools/            spectrogram.py: look at what you can't hear
```

As in the XK-1, the UI never touches the audio engine directly. It sends
small commands (set a parameter, trigger, master volume) over the worklet's
`MessagePort` on the web and over an `mpsc` channel on the desktop.

## Sources and inspirations

- S. Bilbao, *Numerical Sound Synthesis* (2009): the soft bow friction model (CREAK)
- J. L. Kelly and C. C. Lochbaum, "Speech synthesis" (1962); N. Thapen, *Pink Trombone*; K. N. Stevens and A. S. House, "Development of a quantitative description of vowel articulation", JASA 27 (1955) (GLOSSA)
- M. Minnaert, "On musical air-bubbles and the sounds of running water" (1933); K. van den Doel, "Physically-based models for liquid sounds" (ICAD 2004) (POUR)
- H. K. Moffatt, "Euler's disk and its finite-time singularity", *Nature* 404 (2000) (COIN)
- N. B. Tufillaro et al., *An Experimental Approach to Nonlinear Dynamics and Chaos*: the bouncing ball on a vibrating table (BOUNCE)
- K. F. Graff, *Wave Motion in Elastic Solids*: flexural waves in plates (ICE)
- P. C. Peters, "Gravitational radiation and the motion of two point masses" (1964); LIGO, GW150914 (INSPIRAL)
- S. Wolfram, "Statistical mechanics of cellular automata" (1983) (AUTOMATA)
- R. N. Shepard (1964), J.-C. Risset: the endless glissando and the accelerating rhythm (PARADOX)
- I. Müller and P. Strehlow, *Rubber and Rubber Balloons* (2004): the non-monotonic balloon pressure (BALLOON)
- L. Collatz, B. Recamán, P. Nørgård's infinity series, H. Bohlen and J. R. Pierce (MUSICBOX)
- O. Niemitalo, "Hilbert transform: polyphase IIR half-band filters" (SHIFT)
- J. Dattorro, "Effect Design, Part 1: Reverberator and Other Filters", JAES 45(9), 1997 (plate reverb)
