import { createCliRenderer, TextAttributes } from "@opentui/core";
import { createRoot, useKeyboard } from "@opentui/react";
import { useCallback, useMemo, useState } from "react";

import { buildPreviewCommand, buildSynthArgs, nextAction, nextField, type Action, type Field, type SynthForm } from "./app-state";

async function run(command: string[], cwd: string): Promise<{ ok: boolean; text: string }> {
  const proc = Bun.spawn(command, { cwd, stderr: "pipe", stdout: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  return { ok: exitCode === 0, text: (exitCode === 0 ? stdout : stderr).trim() || `exit ${exitCode}` };
}

async function synthesize(form: SynthForm): Promise<{ ok: boolean; text: string }> {
  return run(["cargo", "run", "--quiet", "--manifest-path", "native-synth/Cargo.toml", "--", ...buildSynthArgs(form)], import.meta.dir + "/..");
}

async function play(path: string): Promise<{ ok: boolean; text: string }> {
  for (const player of ["pw-play", "paplay", "aplay", "ffplay"]) {
    if (Bun.which(player)) {
      const args = player === "ffplay" ? ["ffplay", "-nodisp", "-autoexit", path] : [player, path];
      return run(args, import.meta.dir + "/..");
    }
  }
  return { ok: false, text: "No audio player found. Install pw-play, paplay, aplay, or ffplay." };
}

function App() {
  const [focused, setFocused] = useState<Field>("text");
  const [text, setText] = useState("Hello from Supertonic");
  const [lang, setLang] = useState("en");
  const [voice, setVoice] = useState("M1");
  const [speed, setSpeed] = useState("1.0");
  const [status, setStatus] = useState("Idle. Fill fields, press Enter.");
  const [popupOpen, setPopupOpen] = useState(false);
  const [action, setAction] = useState<Action>("play");

  const form = useMemo(
    () => ({
      text,
      lang,
      voice,
      speed,
      output: "supertonic.wav",
    }),
    [lang, speed, text, voice],
  );

  const previewCommand = buildPreviewCommand(form);

  useKeyboard((key) => {
    if (key.name === "escape" || (key.ctrl && key.name === "c")) {
      if (popupOpen && key.name === "escape") {
        setPopupOpen(false);
        return;
      }
      process.exit(0);
    }
    if (popupOpen) {
      if (["tab", "left", "right", "up", "down"].includes(key.name)) {
        setAction((current: Action) => nextAction(current));
      }
      if (key.name === "return") {
        void runAction(action);
      }
      return;
    }
    if (key.name === "tab") {
      setFocused((field: Field) => nextField(field));
    }
  });

  const openPopup = useCallback(() => {
    setPopupOpen(true);
  }, []);

  const runAction = useCallback(async (selected: Action) => {
    setPopupOpen(false);
    const output = selected === "save" ? "supertonic.wav" : "/tmp/supertonic-preview.wav";
    setStatus(selected === "save" ? "Synthesizing to supertonic.wav." : "Synthesizing preview. First run downloads model.");
    const result = await synthesize({ ...form, output });
    if (!result.ok) {
      setStatus(result.text);
      return;
    }
    if (selected === "save") {
      setStatus(result.text);
      return;
    }
    setStatus("Playing preview.");
    const played = await play(output);
    setStatus(played.ok ? "Played preview." : played.text);
  }, [form]);

  return (
    <box style={{ flexGrow: 1, padding: 1, flexDirection: "column", gap: 1 }}>
      <box style={{ flexDirection: "row", justifyContent: "space-between" }}>
        <box style={{ flexDirection: "column" }}>
          <ascii-font font="tiny" text="Supertonic" />
          <text attributes={TextAttributes.DIM}>On-device TTS command builder</text>
        </box>
        <box style={{ flexDirection: "column", alignItems: "flex-end" }}>
          <text fg="#7dd3fc">Tab: focus</text>
          <text fg="#7dd3fc">Enter: choose action</text>
          <text fg="#7dd3fc">Esc/Ctrl+C: quit</text>
        </box>
      </box>

      <box style={{ flexDirection: "row", gap: 1, flexGrow: 1 }}>
        <box title="Synthesis" style={{ border: true, padding: 1, flexDirection: "column", gap: 1, flexGrow: 1 }}>
          <box title="Text" style={{ border: true, height: 3 }}>
            <input value={text} focused={focused === "text" && !popupOpen} onInput={setText} onSubmit={openPopup} />
          </box>
          <box title="Lang" style={{ border: true, height: 3 }}>
            <input value={lang} focused={focused === "lang" && !popupOpen} onInput={setLang} onSubmit={openPopup} />
          </box>
          <box title="Voice" style={{ border: true, height: 3 }}>
            <input value={voice} focused={focused === "voice" && !popupOpen} onInput={setVoice} onSubmit={openPopup} />
          </box>
          <box title="Speed" style={{ border: true, height: 3 }}>
            <input value={speed} focused={focused === "speed" && !popupOpen} onInput={setSpeed} onSubmit={openPopup} />
          </box>
        </box>

        <box title="Status" style={{ border: true, padding: 1, flexDirection: "column", gap: 1, width: 34 }}>
          <text fg="#22c55e">Model</text>
          <text attributes={TextAttributes.DIM}>Supertone/supertonic-3</text>
          <text fg="#22c55e">Output</text>
          <text attributes={TextAttributes.DIM}>{form.output}</text>
          <text fg="#22c55e">State</text>
          <text>{status}</text>
          <text fg="#22c55e">Command</text>
          <text>{previewCommand}</text>
        </box>
      </box>
      {popupOpen ? (
        <box
          title="Synthesize"
          style={{
            border: true,
            padding: 1,
            position: "absolute",
            top: 7,
            left: 18,
            width: 44,
            flexDirection: "column",
            gap: 1,
            backgroundColor: "#111827",
          }}
        >
          <text>Choose output:</text>
          <text fg={action === "play" ? "#22c55e" : "#94a3b8"}>{action === "play" ? "> " : "  "}Hear in app</text>
          <text fg={action === "save" ? "#22c55e" : "#94a3b8"}>{action === "save" ? "> " : "  "}Save WAV file</text>
          <text attributes={TextAttributes.DIM}>Tab/arrows switch. Enter confirms. Esc cancels.</text>
        </box>
      ) : null}
    </box>
  );
}

const renderer = await createCliRenderer();
createRoot(renderer).render(<App />);
