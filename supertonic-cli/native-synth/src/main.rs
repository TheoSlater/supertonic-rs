use st_tts::{SynthesisParams, Tts};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut text = String::new();
    let mut lang = String::from("en");
    let mut voice = String::from("M1");
    let mut speed = 1.0;
    let mut output = String::from("supertonic.wav");

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing value for {arg}"))?;
        match arg.as_str() {
            "--text" => text = value,
            "--lang" => lang = value,
            "--voice" => voice = value,
            "--speed" => speed = value.parse()?,
            "--output" => output = value,
            _ => anyhow::bail!("unknown arg: {arg}"),
        }
    }

    if text.trim().is_empty() {
        anyhow::bail!("text is required");
    }

    let mut params = SynthesisParams::default();
    params.speed = speed;

    let tts = Tts::new("Supertone/supertonic-3", &voice).await?;
    let wav = tts.synthesize_wav(&text, &lang, Some(&params)).await?;
    std::fs::write(&output, wav)?;

    println!("wrote {output}");
    Ok(())
}
