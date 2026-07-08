export type Field = "text" | "lang" | "voice" | "speed";
export type Action = "save" | "play";

const fields: Field[] = ["text", "lang", "voice", "speed"];
const actions: Action[] = ["save", "play"];

export type SynthForm = {
  text: string;
  lang: string;
  voice: string;
  speed: string;
  output: string;
};

export function nextField(field: Field): Field {
  return fields[(fields.indexOf(field) + 1) % fields.length]!;
}

export function nextAction(action: Action): Action {
  return actions[(actions.indexOf(action) + 1) % actions.length]!;
}

export function buildPreviewCommand(form: SynthForm): string {
  return `cargo run --manifest-path native-synth/Cargo.toml -- ${buildSynthArgs(form)
    .map(shellQuote)
    .join(" ")}`;
}

export function buildSynthArgs(form: SynthForm): string[] {
  return ["--text", form.text, "--lang", form.lang, "--voice", form.voice, "--speed", form.speed, "--output", form.output];
}

function shellQuote(value: string): string {
  if (/^[A-Za-z0-9_./:=@-]+$/.test(value)) {
    return value;
  }
  return `"${value.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
}
