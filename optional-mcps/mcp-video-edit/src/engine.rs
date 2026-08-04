//! Engine dispatch for mcp-video-edit.
//!
//! Each edit tool takes an optional `engine` parameter (default `"ffmpeg"`).
//! `resolve_engine` returns the backend; the tool calls the trait method. This
//! keeps engine selection per-call (no env var, no restart, no separate binary)
//! and backward compatible (omitting `engine` = ffmpeg, unchanged behavior).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::ffmpeg;
use crate::tools::{Caption, CaptionStyle, Overlay, Shot};

/// Outcome of a `cut` operation.
#[derive(Debug)]
pub struct CutOutcome {
    pub output_path: PathBuf,
    pub segment_count: usize,
}

/// Outcome of a `render` / `encode_916` operation.
#[derive(Debug)]
pub struct EncodeOutcome {
    pub output_path: PathBuf,
    pub width: u32,
    pub height: u32,
}

/// A video-edit backend. One impl per engine. Methods mirror the existing
/// tool operations and return `Result<_, String>` to match tool error style.
pub trait EditEngine: Send + Sync {
    fn name(&self) -> &'static str;

    fn cut(&self, input: &Path, shots: &[Shot]) -> Result<CutOutcome, String>;
    fn overlay(&self, input: &Path, overlays: Vec<Overlay>) -> Result<PathBuf, String>;
    fn burn_captions(
        &self,
        input: &Path,
        captions: Option<Vec<Caption>>,
        style: Option<CaptionStyle>,
    ) -> Result<PathBuf, String>;
    fn add_music(
        &self,
        input: &Path,
        music: &Path,
        level_db: Option<f64>,
        duck: Option<bool>,
    ) -> Result<PathBuf, String>;
    fn render(&self, input: &Path, quality: Option<String>) -> Result<EncodeOutcome, String>;
    fn concat(&self, inputs: &[PathBuf], transition: Option<String>) -> Result<PathBuf, String>;
    fn extract_audio(&self, input: &Path, format: Option<String>) -> Result<PathBuf, String>;
}

// ---------------------------------------------------------------------------
// ffmpeg engine (default; relocated verbatim from tools.rs)
// ---------------------------------------------------------------------------

pub struct FfmpegEngine;

impl FfmpegEngine {
    fn unique_suffix() -> String {
        std::process::id().to_string()
    }

    fn build_segment_path(input: &Path, idx: usize, suffix: &str) -> PathBuf {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("segment");
        ffmpeg::workspace_dir().join(format!(
            "{}_{}_{}_{}.mp4",
            stem,
            idx,
            suffix,
            Self::unique_suffix()
        ))
    }

    fn position_coords(pos: &str) -> (i32, i32) {
        match pos.to_lowercase().as_str() {
            "top-left" | "topleft" => (10, 10),
            "top-right" | "topright" => (-10, 10),
            "bottom-left" | "bottomleft" => (10, -10),
            "bottom-right" | "bottomright" => (-10, -10),
            "center" => (0, 0),
            _ => (10, 10),
        }
    }

    fn build_cut_args(
        &self,
        input: &Path,
        shots: &[Shot],
    ) -> Result<(PathBuf, Vec<Vec<String>>), String> {
        let suffix = Self::unique_suffix();
        let duration = ffmpeg::ffprobe_duration(input).map_err(|e| e.to_string())?;

        if shots.is_empty() {
            return Err("shots cannot be empty".to_string());
        }
        let mut last_end: f64 = -1.0;
        let mut total: f64 = 0.0;
        for shot in shots {
            if shot.start_sec < 0.0 || shot.end_sec <= shot.start_sec {
                return Err(format!("invalid shot: {}-{}", shot.start_sec, shot.end_sec));
            }
            if shot.start_sec < last_end {
                return Err("shots must not overlap".to_string());
            }
            total += shot.end_sec - shot.start_sec;
            last_end = shot.end_sec;
        }
        if total > duration {
            return Err(format!(
                "shots total {} exceed input duration {}",
                total, duration
            ));
        }

        let mut segment_paths = Vec::new();
        let mut invocations: Vec<Vec<String>> = Vec::new();

        for (idx, shot) in shots.iter().enumerate() {
            let seg_path = Self::build_segment_path(input, idx, &suffix);
            segment_paths.push(seg_path.clone());
            invocations.push(vec![
                "-y".to_string(),
                "-i".to_string(),
                input.to_string_lossy().to_string(),
                "-ss".to_string(),
                shot.start_sec.to_string(),
                "-to".to_string(),
                shot.end_sec.to_string(),
                "-c".to_string(),
                "copy".to_string(),
                seg_path.to_string_lossy().to_string(),
            ]);
        }

        let concat_list =
            ffmpeg::write_concat_list(&segment_paths).map_err(|e| e.to_string())?;
        let output = ffmpeg::output_path(input, &format!("cut_{}", suffix), "mp4");
        invocations.push(vec![
            "-f".to_string(),
            "concat".to_string(),
            "-safe".to_string(),
            "0".to_string(),
            "-i".to_string(),
            concat_list.to_string_lossy().to_string(),
            "-c".to_string(),
            "copy".to_string(),
            output.to_string_lossy().to_string(),
        ]);

        Ok((output, invocations))
    }

    fn build_burn_args(
        &self,
        input: &Path,
        captions: Option<Vec<Caption>>,
        _style: Option<CaptionStyle>,
    ) -> Result<(PathBuf, Vec<String>), String> {
        let output =
            ffmpeg::output_path(input, &format!("burn_{}", Self::unique_suffix()), "mp4");
        let mut args = vec![
            "-y".to_string(),
            "-i".to_string(),
            input.to_string_lossy().to_string(),
        ];

        if let Some(caps) = captions {
            if !caps.is_empty() {
                let ass_path =
                    ffmpeg::workspace_dir().join(format!("captions_{}.ass", Self::unique_suffix()));
                let tuples: Vec<(f64, f64, String)> = caps
                    .into_iter()
                    .map(|c| (c.start_sec, c.end_sec, c.text))
                    .collect();
                ffmpeg::write_ass(&tuples, &ass_path).map_err(|e| e.to_string())?;
                args.push("-vf".to_string());
                args.push(format!("ass={}", ass_path.to_string_lossy()));
            }
        } else {
            // Auto-transcribe via whisper.
            let ws = ffmpeg::ensure_workspace().map_err(|e| e.to_string())?;
            let srt =
                ffmpeg::whisper(input, &ffmpeg::whisper_model(), &ws).map_err(|e| e.to_string())?;
            args.push("-vf".to_string());
            args.push(format!("subtitles={}", srt.to_string_lossy()));
        }

        args.push("-c:a".to_string());
        args.push("copy".to_string());
        args.push(output.to_string_lossy().to_string());
        Ok((output, args))
    }

    fn build_overlay_args(
        &self,
        input: &Path,
        overlays: Vec<Overlay>,
    ) -> Result<(PathBuf, Vec<String>), String> {
        let output =
            ffmpeg::output_path(input, &format!("overlay_{}", Self::unique_suffix()), "mp4");
        let mut filters: Vec<String> = Vec::new();
        for ov in overlays {
            let (x, y) = Self::position_coords(&ov.position);
            let x_expr = if x < 0 {
                format!("w-tw{}", x)
            } else {
                x.to_string()
            };
            let y_expr = if y < 0 {
                format!("h-th{}", y)
            } else {
                y.to_string()
            };
            filters.push(format!(
                "drawtext=text='{}':enable='between(t,{},{})':x={}:y={}:fontsize=24:fontcolor=white:box=1:boxcolor=black@0.5",
                ov.text.replace("'", "'\\\\''"),
                ov.start_sec,
                ov.end_sec,
                x_expr,
                y_expr
            ));
        }

        let args = vec![
            "-y".to_string(),
            "-i".to_string(),
            input.to_string_lossy().to_string(),
            "-vf".to_string(),
            filters.join(","),
            "-c:a".to_string(),
            "copy".to_string(),
            output.to_string_lossy().to_string(),
        ];
        Ok((output, args))
    }

    fn build_add_music_args(
        &self,
        input: &Path,
        music: &Path,
        level_db: Option<f64>,
        duck: Option<bool>,
    ) -> Result<(PathBuf, Vec<String>), String> {
        let output =
            ffmpeg::output_path(input, &format!("music_{}", Self::unique_suffix()), "mp4");
        let level = level_db.unwrap_or(-20.0);
        let filter = if duck.unwrap_or(false) {
            format!(
                "[0:a][1:a]amix=inputs=2:duration=first:dropout_transition=2[amix];[amix]volume={}dB[aout]",
                level
            )
        } else {
            format!(
                "[0:a][1:a]amix=inputs=2:duration=first:dropout_transition=2,volume={}dB[aout]",
                level
            )
        };
        let args = vec![
            "-y".to_string(),
            "-i".to_string(),
            input.to_string_lossy().to_string(),
            "-i".to_string(),
            music.to_string_lossy().to_string(),
            "-filter_complex".to_string(),
            filter,
            "-map".to_string(),
            "0:v".to_string(),
            "-map".to_string(),
            "[aout]".to_string(),
            "-c:v".to_string(),
            "copy".to_string(),
            output.to_string_lossy().to_string(),
        ];
        Ok((output, args))
    }

    fn build_encode_916_args(
        &self,
        input: &Path,
        quality: Option<String>,
    ) -> Result<(PathBuf, Vec<String>), String> {
        let output =
            ffmpeg::output_path(input, &format!("916_{}", Self::unique_suffix()), "mp4");
        let crf = match quality.as_deref() {
            Some("high") => "18",
            Some("medium") => "23",
            Some("low") => "28",
            _ => "23",
        };
        let filter =
            "scale=1080:1920:force_original_aspect_ratio=decrease,setsar=1,crop=1080:1920"
                .to_string();
        let args = vec![
            "-y".to_string(),
            "-i".to_string(),
            input.to_string_lossy().to_string(),
            "-vf".to_string(),
            filter,
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "fast".to_string(),
            "-crf".to_string(),
            crf.to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
            output.to_string_lossy().to_string(),
        ];
        Ok((output, args))
    }

    fn build_concat_args(
        &self,
        inputs: &[PathBuf],
        transition: Option<String>,
    ) -> Result<(PathBuf, Vec<String>), String> {
        let output = ffmpeg::workspace_dir().join(format!("concat_{}.mp4", Self::unique_suffix()));
        if let Some(t) = transition {
            let mut args = vec!["-y".to_string()];
            for input in inputs {
                args.push("-i".to_string());
                args.push(input.to_string_lossy().to_string());
            }
            let mut filter_parts = Vec::new();
            for (i, _input) in inputs.iter().enumerate() {
                filter_parts.push(format!("[{}:v]format=yuv420p[v{}]", i, i));
            }
            let mut chain = "[v0]".to_string();
            for i in 1..inputs.len() {
                chain = format!("{}[v{}]xfade=transition={}:duration=0.5[tmp{}]", chain, i, t, i);
                if i < inputs.len() - 1 {
                    chain = format!("[tmp{}]", i);
                }
            }
            filter_parts.push(chain);
            let filter = filter_parts.join(";");
            args.extend(vec![
                "-filter_complex".to_string(),
                filter,
                "-map".to_string(),
                format!("[tmp{}]", inputs.len() - 1),
                output.to_string_lossy().to_string(),
            ]);
            Ok((output, args))
        } else {
            let concat_list = ffmpeg::write_concat_list(inputs).map_err(|e| e.to_string())?;
            let args = vec![
                "-y".to_string(),
                "-f".to_string(),
                "concat".to_string(),
                "-safe".to_string(),
                "0".to_string(),
                "-i".to_string(),
                concat_list.to_string_lossy().to_string(),
                "-c".to_string(),
                "copy".to_string(),
                output.to_string_lossy().to_string(),
            ];
            Ok((output, args))
        }
    }

    fn build_extract_audio_args(
        &self,
        input: &Path,
        format: Option<String>,
    ) -> Result<(PathBuf, Vec<String>), String> {
        let ext = match format.as_deref() {
            Some("mp3") => "mp3",
            Some("wav") => "wav",
            _ => "m4a",
        };
        let codec = match ext {
            "mp3" => "libmp3lame",
            "wav" => "pcm_s16le",
            _ => "aac",
        };
        let output =
            ffmpeg::output_path(input, &format!("audio_{}", Self::unique_suffix()), ext);
        let args = vec![
            "-y".to_string(),
            "-i".to_string(),
            input.to_string_lossy().to_string(),
            "-vn".to_string(),
            "-acodec".to_string(),
            codec.to_string(),
            output.to_string_lossy().to_string(),
        ];
        Ok((output, args))
    }
}

impl EditEngine for FfmpegEngine {
    fn name(&self) -> &'static str {
        "ffmpeg"
    }

    fn cut(&self, input: &Path, shots: &[Shot]) -> Result<CutOutcome, String> {
        let (output, invocations) = self.build_cut_args(input, shots)?;
        for args in invocations {
            ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .map_err(|e| e.to_string())?;
        }
        Ok(CutOutcome {
            output_path: output,
            segment_count: shots.len(),
        })
    }

    fn overlay(&self, input: &Path, overlays: Vec<Overlay>) -> Result<PathBuf, String> {
        let (output, args) = self.build_overlay_args(input, overlays)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        Ok(output)
    }

    fn burn_captions(
        &self,
        input: &Path,
        captions: Option<Vec<Caption>>,
        style: Option<CaptionStyle>,
    ) -> Result<PathBuf, String> {
        let (output, args) = self.build_burn_args(input, captions, style)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        Ok(output)
    }

    fn add_music(
        &self,
        input: &Path,
        music: &Path,
        level_db: Option<f64>,
        duck: Option<bool>,
    ) -> Result<PathBuf, String> {
        let (output, args) = self.build_add_music_args(input, music, level_db, duck)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        Ok(output)
    }

    fn render(&self, input: &Path, quality: Option<String>) -> Result<EncodeOutcome, String> {
        let (output, args) = self.build_encode_916_args(input, quality)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        Ok(EncodeOutcome {
            output_path: output,
            width: 1080,
            height: 1920,
        })
    }

    fn concat(&self, inputs: &[PathBuf], transition: Option<String>) -> Result<PathBuf, String> {
        let (output, args) = self.build_concat_args(inputs, transition)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        Ok(output)
    }

    fn extract_audio(&self, input: &Path, format: Option<String>) -> Result<PathBuf, String> {
        let (output, args) = self.build_extract_audio_args(input, format)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        Ok(output)
    }
}

// ---------------------------------------------------------------------------
// DaVinci engine (drives live DaVinci Resolve via the python bridge)
// ---------------------------------------------------------------------------

pub struct DavinciEngine;

/// Path to the python bridge script. Resolution order:
/// 1. `DAVINCI_BRIDGE` env (explicit override)
/// 2. candidate locations relative to the current executable
/// 3. `davinci_bridge.py` in CWD / PATH
fn bridge_path() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("DAVINCI_BRIDGE") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
        return Err(format!(
            "DAVINCI_BRIDGE set but script not found: {}",
            p.display()
        ));
    }
    let mut candidates: Vec<PathBuf> = vec![PathBuf::from("davinci_bridge.py")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("scripts").join("davinci_bridge.py"));
            candidates.push(parent.join("davinci_bridge.py"));
            if let Some(gp) = parent.parent() {
                candidates.push(gp.join("scripts").join("davinci_bridge.py"));
            }
        }
    }
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    Err(format!(
        "davinci_bridge.py not found (tried {:?}); set DAVINCI_BRIDGE to its path",
        candidates
    ))
}

fn python_bin() -> PathBuf {
    std::env::var("DAVINCI_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("python3"))
}

/// Run the python bridge for one command. Returns the parsed JSON object on
/// success, or the bridge's `error` field / stderr as `Err`.
fn davinci_run(command: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let bridge = bridge_path()?;
    let mut cmd = Command::new(python_bin());
    cmd.arg(&bridge).arg(command);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn davinci bridge ({}): {}", python_bin().display(), e))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "no stdin to davinci bridge".to_string())?;
        stdin
            .write_all(args.to_string().as_bytes())
            .map_err(|e| format!("write bridge stdin: {}", e))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait davinci bridge: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    // Bridge always prints a JSON object on stdout, even on failure.
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("parse davinci bridge output: {e}: stdout={stdout} stderr={stderr}"))?;
    if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    if !output.status.success() && parsed.get("output_path").is_none() && parsed.get("ok").is_none()
    {
        return Err(format!(
            "davinci bridge exited {} with no result: stderr={}",
            output.status, stderr
        ));
    }
    Ok(parsed)
}

fn path_from(v: &serde_json::Value, key: &str) -> Result<PathBuf, String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(PathBuf::from)
        .ok_or_else(|| format!("davinci bridge missing `{key}` in response: {v}"))
}

impl EditEngine for DavinciEngine {
    fn name(&self) -> &'static str {
        "davinci"
    }

    fn cut(&self, input: &Path, shots: &[Shot]) -> Result<CutOutcome, String> {
        let args = serde_json::json!({
            "input": input.to_string_lossy(),
            "shots": shots,
        });
        let v = davinci_run("cut", &args)?;
        let output_path = path_from(&v, "output_path")?;
        let segment_count = v
            .get("segment_count")
            .and_then(|x| x.as_u64())
            .unwrap_or(shots.len() as u64) as usize;
        Ok(CutOutcome {
            output_path,
            segment_count,
        })
    }

    fn overlay(&self, input: &Path, overlays: Vec<Overlay>) -> Result<PathBuf, String> {
        let args = serde_json::json!({
            "input": input.to_string_lossy(),
            "overlays": overlays,
        });
        let v = davinci_run("overlay", &args)?;
        path_from(&v, "output_path")
    }

    fn burn_captions(
        &self,
        input: &Path,
        captions: Option<Vec<Caption>>,
        style: Option<CaptionStyle>,
    ) -> Result<PathBuf, String> {
        let args = serde_json::json!({
            "input": input.to_string_lossy(),
            "captions": captions,
            "style": style,
        });
        let v = davinci_run("burn_captions", &args)?;
        path_from(&v, "output_path")
    }

    fn add_music(
        &self,
        input: &Path,
        music: &Path,
        level_db: Option<f64>,
        duck: Option<bool>,
    ) -> Result<PathBuf, String> {
        let args = serde_json::json!({
            "input": input.to_string_lossy(),
            "music": music.to_string_lossy(),
            "level_db": level_db,
            "duck": duck,
        });
        let v = davinci_run("add_music", &args)?;
        path_from(&v, "output_path")
    }

    fn render(&self, input: &Path, quality: Option<String>) -> Result<EncodeOutcome, String> {
        let args = serde_json::json!({
            "input": input.to_string_lossy(),
            "quality": quality,
            // prores works on Linux free + Studio; Windows free may override via env
            // DAVINCI_CODEC=h264 if needed (bridge reads args.get("codec")).
            "codec": std::env::var("DAVINCI_CODEC").unwrap_or_else(|_| "prores".to_string()),
        });
        let v = davinci_run("render", &args)?;
        let output_path = path_from(&v, "output_path")?;
        let width = v.get("width").and_then(|x| x.as_u64()).unwrap_or(1080) as u32;
        let height = v.get("height").and_then(|x| x.as_u64()).unwrap_or(1920) as u32;
        Ok(EncodeOutcome {
            output_path,
            width,
            height,
        })
    }

    fn concat(&self, inputs: &[PathBuf], transition: Option<String>) -> Result<PathBuf, String> {
        let args = serde_json::json!({
            "inputs": inputs.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
            "transition": transition,
        });
        let v = davinci_run("concat", &args)?;
        path_from(&v, "output_path")
    }

    fn extract_audio(&self, input: &Path, format: Option<String>) -> Result<PathBuf, String> {
        let args = serde_json::json!({
            "input": input.to_string_lossy(),
            "format": format,
        });
        let v = davinci_run("extract_audio", &args)?;
        path_from(&v, "output_path")
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Resolve the engine for a single tool call.
///
/// - `None` | `"ffmpeg"` → ffmpeg (default; backward compatible).
/// - `"davinci"` → davinci (drives live Resolve via the python bridge).
/// - unknown → ffmpeg (lenient: treat unknown as the default).
pub fn resolve_engine(engine: Option<&str>) -> Box<dyn EditEngine> {
    match engine {
        Some("davinci") => Box::new(DavinciEngine),
        _ => Box::new(FfmpegEngine),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize env-mutating davinci tests against each other + bridge_path test.
    static DAVINCI_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_input() -> PathBuf {
        let dir = tempfile::tempdir().unwrap();
        dir.path().join("test_input.mp4")
    }

    #[test]
    fn test_encode_916_args_shape() {
        let eng = FfmpegEngine;
        let input = temp_input();
        let (_output, args) = eng.build_encode_916_args(&input, None).unwrap();
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.iter().any(|a| a.contains("1080:1920")));
    }

    #[test]
    fn test_extract_audio_args_shape() {
        let eng = FfmpegEngine;
        let input = temp_input();
        let (_output, args) = eng.build_extract_audio_args(&input, Some("mp3".to_string())).unwrap();
        assert!(args.contains(&"libmp3lame".to_string()));
        assert!(args.contains(&"-vn".to_string()));
    }

    #[test]
    fn test_bridge_path_missing_errors() {
        // Env-mutating; serialize against the davinci mock tests below.
        let _guard = DAVINCI_ENV_LOCK.lock().unwrap();
        let saved = std::env::var("DAVINCI_BRIDGE").ok();
        std::env::set_var("DAVINCI_BRIDGE", "/no/such/davinci_bridge.py");
        let err = bridge_path().unwrap_err();
        assert!(err.contains("DAVINCI_BRIDGE"), "got: {err}");
        // restore
        match saved {
            Some(v) => std::env::set_var("DAVINCI_BRIDGE", v),
            None => std::env::remove_var("DAVINCI_BRIDGE"),
        }
    }

    // Mock "python" as a shell script: reads command from $1, ignores stdin,
    // prints canned JSON. Lets us exercise the real subprocess + JSON parsing
    // path without a live Resolve. Unix-only (sh + exec bit).
    #[cfg(unix)]
    fn write_mock_python(dir: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let script = r#"#!/bin/sh
# argv: [mockpython, <bridge_path>, <command>]
cmd="$2"
cat >/dev/null
case "$cmd" in
  cut)            echo '{"output_path":"timeline:mock","segment_count":2,"engine":"davinci"}';;
  overlay)        echo '{"output_path":"timeline:mock","engine":"davinci"}';;
  burn_captions)  echo '{"output_path":"timeline:mock","engine":"davinci"}';;
  add_music)      echo '{"output_path":"timeline:mock","engine":"davinci"}';;
  render)         echo '{"output_path":"/tmp/r.mov","width":1080,"height":1920,"engine":"davinci"}';;
  concat)         echo '{"output_path":"timeline:mock","engine":"davinci"}';;
  extract_audio)  echo '{"output_path":"/tmp/a.m4a","engine":"davinci"}';;
  health)         echo '{"ok":true,"engine":"davinci","name":"Resolve"}';;
  failcmd)        echo '{"error":"forced failure"}'; exit 1;;
  *)              echo '{"error":"unknown command: '"$cmd"'"}'; exit 2;;
esac
"#;
        let p = dir.join("mockpython");
        std::fs::write(&p, script).unwrap();
        let mut perms = std::fs::metadata(&p).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&p, perms).unwrap();
        p
    }

    #[cfg(unix)]
    fn setup_davinci_mocks() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let mock_py = write_mock_python(dir.path());
        std::env::set_var("DAVINCI_PYTHON", &mock_py);
        // bridge_path needs an existing file; the mock python ignores the script arg.
        let dummy_bridge = dir.path().join("davinci_bridge.py");
        std::fs::write(&dummy_bridge, "# mock bridge\n").unwrap();
        std::env::set_var("DAVINCI_BRIDGE", &dummy_bridge);
        (dir, mock_py)
    }

    #[cfg(unix)]
    fn teardown_davinci_env(saved_py: Option<String>, saved_bridge: Option<String>) {
        match saved_py {
            Some(v) => std::env::set_var("DAVINCI_PYTHON", v),
            None => std::env::remove_var("DAVINCI_PYTHON"),
        }
        match saved_bridge {
            Some(v) => std::env::set_var("DAVINCI_BRIDGE", v),
            None => std::env::remove_var("DAVINCI_BRIDGE"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_davinci_run_cut_success() {
        let _guard = DAVINCI_ENV_LOCK.lock().unwrap();
        let saved_py = std::env::var("DAVINCI_PYTHON").ok();
        let saved_bridge = std::env::var("DAVINCI_BRIDGE").ok();
        let (_dir, _mock) = setup_davinci_mocks();
        let v = davinci_run("cut", &serde_json::json!({"input":"/tmp/x.mp4"})).unwrap();
        assert_eq!(v["output_path"], "timeline:mock");
        assert_eq!(v["segment_count"], 2);
        assert_eq!(v["engine"], "davinci");
        teardown_davinci_env(saved_py, saved_bridge);
    }

    #[cfg(unix)]
    #[test]
    fn test_davinci_run_error_propagates() {
        let _guard = DAVINCI_ENV_LOCK.lock().unwrap();
        let saved_py = std::env::var("DAVINCI_PYTHON").ok();
        let saved_bridge = std::env::var("DAVINCI_BRIDGE").ok();
        let (_dir, _mock) = setup_davinci_mocks();
        let err = davinci_run("failcmd", &serde_json::json!({})).unwrap_err();
        assert_eq!(err, "forced failure");
        teardown_davinci_env(saved_py, saved_bridge);
    }

    #[cfg(unix)]
    #[test]
    fn test_davinci_run_unknown_command() {
        let _guard = DAVINCI_ENV_LOCK.lock().unwrap();
        let saved_py = std::env::var("DAVINCI_PYTHON").ok();
        let saved_bridge = std::env::var("DAVINCI_BRIDGE").ok();
        let (_dir, _mock) = setup_davinci_mocks();
        let err = davinci_run("bogus", &serde_json::json!({})).unwrap_err();
        assert!(err.contains("unknown command"), "got: {err}");
        teardown_davinci_env(saved_py, saved_bridge);
    }

    #[cfg(unix)]
    #[test]
    fn test_davinci_engine_cut_mock() {
        let _guard = DAVINCI_ENV_LOCK.lock().unwrap();
        let saved_py = std::env::var("DAVINCI_PYTHON").ok();
        let saved_bridge = std::env::var("DAVINCI_BRIDGE").ok();
        let (_dir, _mock) = setup_davinci_mocks();
        let eng = DavinciEngine;
        let shots = vec![
            Shot { start_sec: 0.0, end_sec: 3.0, label: "a".to_string() },
            Shot { start_sec: 3.0, end_sec: 10.0, label: "b".to_string() },
        ];
        let outcome = eng.cut(Path::new("/tmp/x.mp4"), &shots).unwrap();
        assert_eq!(outcome.output_path, PathBuf::from("timeline:mock"));
        assert_eq!(outcome.segment_count, 2);
        teardown_davinci_env(saved_py, saved_bridge);
    }

    #[cfg(unix)]
    #[test]
    fn test_davinci_engine_render_mock() {
        let _guard = DAVINCI_ENV_LOCK.lock().unwrap();
        let saved_py = std::env::var("DAVINCI_PYTHON").ok();
        let saved_bridge = std::env::var("DAVINCI_BRIDGE").ok();
        let (_dir, _mock) = setup_davinci_mocks();
        let eng = DavinciEngine;
        let outcome = eng.render(Path::new("/tmp/x.mp4"), Some("medium".to_string())).unwrap();
        assert_eq!(outcome.output_path, PathBuf::from("/tmp/r.mov"));
        assert_eq!(outcome.width, 1080);
        assert_eq!(outcome.height, 1920);
        teardown_davinci_env(saved_py, saved_bridge);
    }
}