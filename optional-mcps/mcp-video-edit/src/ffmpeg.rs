use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn workspace_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("EDIT_WORKSPACE") {
        PathBuf::from(dir)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".hermes/work")
    }
}

pub fn ensure_workspace() -> Result<PathBuf> {
    let dir = workspace_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating workspace {}", dir.display()))?;
    Ok(dir)
}

pub fn output_path(input: &Path, suffix: &str, ext: &str) -> PathBuf {
    let ws = workspace_dir();
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let name = format!("{}_{}.{}", stem, suffix, ext);
    ws.join(name)
}

pub fn ffmpeg_bin() -> PathBuf {
    std::env::var("EDIT_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ffmpeg"))
}

pub fn ffprobe_bin() -> PathBuf {
    std::env::var("EDIT_FFPROBE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ffprobe"))
}

pub fn whisper_bin() -> PathBuf {
    std::env::var("WHISPER_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("whisper-cli"))
}

pub fn whisper_model() -> PathBuf {
    if let Ok(path) = std::env::var("WHISPER_MODEL") {
        PathBuf::from(path)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".hermes/models/ggml-base.bin")
    }
}

pub fn run(bin: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd.output().with_context(|| {
        format!("failed to execute {} {}", bin.display(), args.join(" "))
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        anyhow::bail!(
            "{} exited with {}: stderr={}",
            bin.display(),
            output.status,
            stderr
        );
    }
    tracing::debug!(%stdout, %stderr, "{} completed", bin.display());
    Ok(stdout)
}

pub fn ffmpeg(args: &[&str]) -> Result<String> {
    run(&ffmpeg_bin(), args)
}

pub fn ffprobe(args: &[&str]) -> Result<String> {
    run(&ffprobe_bin(), args)
}

pub fn ffprobe_duration(input: &Path) -> Result<f64> {
    let out = ffprobe(&[
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
        &input.to_string_lossy(),
    ])?;
    out.trim()
        .parse::<f64>()
        .with_context(|| format!("parsing ffprobe duration: {}", out))
}

pub fn whisper(input: &Path, model: &Path, output_dir: &Path) -> Result<PathBuf> {
    let bin = whisper_bin();
    let out = run(&bin, &[
        "-m",
        &model.to_string_lossy(),
        "-f",
        &input.to_string_lossy(),
        "-of",
        &output_dir.join("transcript").to_string_lossy(),
        "--output-srt",
    ])?;
    tracing::debug!(%out, "whisper completed");
    let srt = output_dir.join("transcript.srt");
    Ok(srt)
}

/// Write a concat demuxer list file and return its path.
pub fn write_concat_list(inputs: &[PathBuf]) -> Result<PathBuf> {
    let ws = ensure_workspace()?;
    let path = ws.join(format!("concat_{}.txt", std::process::id()));
    let mut file = std::fs::File::create(&path)?;
    for input in inputs {
        writeln!(file, "file '{}'", input.to_string_lossy())?;
    }
    Ok(path)
}

/// Build a simple ASS subtitle string from caption entries.
pub fn build_ass(captions: &[(f64, f64, String)], width: u32, height: u32) -> String {
    let mut ass = format!(
        "[Script Info]\nTitle: auto\nScriptType: v4.00+\nPlayResX: {}\nPlayResY: {}\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,24,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,10,1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        width, height
    );
    for (start, end, text) in captions {
        ass.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
            fmt_time(*start),
            fmt_time(*end),
            text.replace(",", "\\,")
        ));
    }
    ass
}

fn fmt_time(seconds: f64) -> String {
    let h = (seconds / 3600.0) as u32;
    let m = ((seconds % 3600.0) / 60.0) as u32;
    let s = seconds % 60.0;
    format!("{:01}:{:02}:{:05.2}", h, m, s)
}

pub fn write_ass(captions: &[(f64, f64, String)], path: &Path) -> Result<()> {
    std::fs::write(path, build_ass(captions, 1920, 1080))?;
    Ok(())
}
