# supertonic-cli

OpenTUI app for synthesizing Supertonic speech from the terminal.

## Run

Requires Bun, Rust, and one playback command: `pw-play`, `paplay`, `aplay`, or `ffplay`.

```bash
bun install
bun run src/index.tsx
```

## Controls

```text
Tab          move between fields
Enter        open synthesize popup
Tab/arrows   switch between Hear in app and Save WAV file
Enter        confirm popup choice
Esc          cancel popup or quit
Ctrl+C       quit
```

First synth downloads `Supertone/supertonic-3`. Save mode writes `supertonic.wav`; hear mode writes a temporary preview and plays it.
