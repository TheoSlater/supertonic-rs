import { expect, test } from "bun:test";
import { buildPreviewCommand, buildSynthArgs, nextAction, nextField, type Field } from "./app-state";

test("nextField cycles through CLI inputs", () => {
  const fields: Field[] = ["text", "lang", "voice", "speed"];

  expect(nextField("text")).toBe("lang");
  expect(nextField("speed")).toBe("text");
  expect(fields.map(nextField)).toEqual(["lang", "voice", "speed", "text"]);
});

test("buildSynthArgs passes TTS options to native helper", () => {
  const form = {
    text: "Hello world",
    lang: "en",
    voice: "M1",
    speed: "1.0",
    output: "hello.wav",
  };

  expect(buildSynthArgs(form)).toEqual([
    "--text",
    "Hello world",
    "--lang",
    "en",
    "--voice",
    "M1",
    "--speed",
    "1.0",
    "--output",
    "hello.wav",
  ]);
  expect(buildPreviewCommand(form)).toBe(
    'cargo run --manifest-path native-synth/Cargo.toml -- --text "Hello world" --lang en --voice M1 --speed 1.0 --output hello.wav',
  );
});

test("nextAction toggles popup choice", () => {
  expect(nextAction("save")).toBe("play");
  expect(nextAction("play")).toBe("save");
});
