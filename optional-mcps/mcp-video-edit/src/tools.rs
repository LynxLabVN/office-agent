use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::ffmpeg;

#[derive(Clone)]
pub struct VideoEditServer;

#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct Shot {
    pub start_sec: f64,
    pub end_sec: f64,
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct Caption {
    pub start_sec: f64,
    pub end_sec: f64,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Default, schemars::JsonSchema)]
pub struct CaptionStyle {
    pub font: Option<String>,
    pub size: Option<i32>,
    pub color: Option<String>,
    pub bg: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct Overlay {
    pub start_sec: f64,
    pub end_sec: f64,
    pub text: String,
    pub position: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OutputPathResponse {
    output_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CutResponse {
    output_path: String,
    segment_count: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct EncodeResponse {
    output_path: String,
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExtractAudioResponse {
    output_path: String,
}

fn unique_suffix() -> String {
    std::process::id().to_string()
}

fn ensure_output_dir() -> Result<PathBuf, String> {
    ffmpeg::ensure_workspace().map_err(|e| e.to_string())
}

fn input_path(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(format!("input not found: {}", p.display()));
    }
    Ok(p)
}

fn build_segment_path(input: &Path, idx: usize, suffix: &str) -> PathBuf {
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("segment");
    ffmpeg::workspace_dir().join(format!("{}_{}_{}_{}.mp4", stem, idx, suffix, unique_suffix()))
}

fn build_cut_args(input: &Path, shots: &[Shot]) -> Result<(PathBuf, Vec<Vec<String>>), String> {
    let suffix = unique_suffix();
    let duration = ffmpeg::ffprobe_duration(input).map_err(|e| e.to_string())?;

    // Validate shots.
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
        let seg_path = build_segment_path(input, idx, &suffix);
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

    let concat_list = ffmpeg::write_concat_list(&segment_paths).map_err(|e| e.to_string())?;
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
    input: &Path,
    captions: Option<Vec<Caption>>,
    _style: Option<CaptionStyle>,
) -> Result<(PathBuf, Vec<String>), String> {
    let output = ffmpeg::output_path(input, &format!("burn_{}", unique_suffix()), "mp4");
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
    ];

    if let Some(caps) = captions {
        if !caps.is_empty() {
            let ass_path = ffmpeg::workspace_dir().join(format!("captions_{}.ass", unique_suffix()));
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
        let srt = ffmpeg::whisper(input, &ffmpeg::whisper_model(), &ws).map_err(|e| e.to_string())?;
        args.push("-vf".to_string());
        args.push(format!("subtitles={}", srt.to_string_lossy()));
    }

    args.push("-c:a".to_string());
    args.push("copy".to_string());
    args.push(output.to_string_lossy().to_string());
    Ok((output, args))
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

fn build_overlay_args(input: &Path, overlays: Vec<Overlay>) -> Result<(PathBuf, Vec<String>), String> {
    let output = ffmpeg::output_path(input, &format!("overlay_{}", unique_suffix()), "mp4");
    let mut filters: Vec<String> = Vec::new();
    for ov in overlays {
        let (x, y) = position_coords(&ov.position);
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
    input: &Path,
    music: &Path,
    level_db: Option<f64>,
    duck: Option<bool>,
) -> Result<(PathBuf, Vec<String>), String> {
    let output = ffmpeg::output_path(input, &format!("music_{}", unique_suffix()), "mp4");
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

fn build_encode_916_args(input: &Path, quality: Option<String>) -> Result<(PathBuf, Vec<String>), String> {
    let output = ffmpeg::output_path(input, &format!("916_{}", unique_suffix()), "mp4");
    let crf = match quality.as_deref() {
        Some("high") => "18",
        Some("medium") => "23",
        Some("low") => "28",
        _ => "23",
    };
    // Scale to fit within 1080x1920 while preserving aspect ratio, then crop to exact 1080x1920.
    let filter = "scale=1080:1920:force_original_aspect_ratio=decrease,setsar=1,crop=1080:1920".to_string();
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

fn build_concat_args(inputs: &[PathBuf], transition: Option<String>) -> Result<(PathBuf, Vec<String>), String> {
    let output = ffmpeg::workspace_dir().join(format!("concat_{}.mp4", unique_suffix()));
    if let Some(t) = transition {
        // xfade transition: build filter_complex with all inputs.
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

fn build_extract_audio_args(input: &Path, format: Option<String>) -> Result<(PathBuf, Vec<String>), String> {
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
    let output = ffmpeg::output_path(input, &format!("audio_{}", unique_suffix()), ext);
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

#[tool(tool_box)]
impl VideoEditServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-video-edit".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "cut_by_shotlist", description = "Cut a video into segments by shot list and concat them")]
    pub fn cut_by_shotlist(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] shots: Vec<Shot>,
    ) -> Result<String, String> {
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let (output, invocations) = build_cut_args(&input, &shots)?;
        for args in invocations {
            ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .map_err(|e| e.to_string())?;
        }
        serde_json::to_string(&CutResponse {
            output_path: output.to_string_lossy().to_string(),
            segment_count: shots.len(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "burn_captions", description = "Burn subtitles or auto-transcribe captions into a video")]
    pub fn burn_captions(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] captions: Option<Vec<Caption>>,
        #[tool(param)] style: Option<CaptionStyle>,
    ) -> Result<String, String> {
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let (output, args) = build_burn_args(&input, captions, style)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&OutputPathResponse {
            output_path: output.to_string_lossy().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "overlay_text", description = "Overlay text on a video using drawtext")]
    pub fn overlay_text(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] overlays: Vec<Overlay>,
    ) -> Result<String, String> {
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let (output, args) = build_overlay_args(&input, overlays)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&OutputPathResponse {
            output_path: output.to_string_lossy().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "add_music", description = "Add background music to a video")]
    pub fn add_music(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] music: String,
        #[tool(param)] level_db: Option<f64>,
        #[tool(param)] duck: Option<bool>,
    ) -> Result<String, String> {
        let input = input_path(&input)?;
        let music = input_path(&music)?;
        ensure_output_dir()?;
        let (output, args) = build_add_music_args(&input, &music, level_db, duck)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&OutputPathResponse {
            output_path: output.to_string_lossy().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "encode_916", description = "Encode a video to 9:16 vertical 1080x1920")]
    pub fn encode_916(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] quality: Option<String>,
    ) -> Result<String, String> {
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let (output, args) = build_encode_916_args(&input, quality)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&EncodeResponse {
            output_path: output.to_string_lossy().to_string(),
            width: 1080,
            height: 1920,
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "concat", description = "Concatenate multiple videos")]
    pub fn concat(
        &self,
        #[tool(param)] inputs: Vec<String>,
        #[tool(param)] transition: Option<String>,
    ) -> Result<String, String> {
        let inputs: Result<Vec<_>, _> = inputs.iter().map(|p| input_path(p)).collect();
        let inputs = inputs?;
        ensure_output_dir()?;
        let (output, args) = build_concat_args(&inputs, transition)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&OutputPathResponse {
            output_path: output.to_string_lossy().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "extract_audio", description = "Extract audio track from a video")]
    pub fn extract_audio(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] format: Option<String>,
    ) -> Result<String, String> {
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let (output, args) = build_extract_audio_args(&input, format)?;
        ffmpeg::ffmpeg(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&ExtractAudioResponse {
            output_path: output.to_string_lossy().to_string(),
        })
        .map_err(|e| e.to_string())
    }
}

impl ServerHandler for VideoEditServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-video-edit".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("FFmpeg video editing MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    // Env vars are process-global; serialize tests that mutate them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn mock_bin(dir: &Path, name: &str, script: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(script.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).unwrap();
        }
        path
    }

    fn setup_mocks() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();

        let ffmpeg_script = r#"#!/bin/sh
# Find output path: the argument after the last -i or after -c copy etc.
# Simplistic: the last argument that does not start with '-' is the output.
out=""
for arg in "$@"; do
    case "$arg" in
        -*) ;;
        *) out="$arg" ;;
    esac
done
if [ -n "$out" ]; then
    mkdir -p "$(dirname "$out")"
    echo "mock" > "$out"
fi
"#;
        let ffprobe_script = r#"#!/bin/sh
echo "10.0"
"#;

        let ffmpeg = mock_bin(&dir_path, "ffmpeg", ffmpeg_script);
        let ffprobe = mock_bin(&dir_path, "ffprobe", ffprobe_script);
        std::env::set_var("EDIT_FFMPEG", &ffmpeg);
        std::env::set_var("EDIT_FFPROBE", &ffprobe);
        (dir, ffmpeg, ffprobe)
    }

    fn test_server() -> VideoEditServer {
        VideoEditServer
    }

    fn temp_input() -> PathBuf {
        let ws = ffmpeg::ensure_workspace().unwrap();
        let p = ws.join(format!("test_input_{}.mp4", std::process::id()));
        std::fs::write(&p, b"fake mp4").unwrap();
        p
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-video-edit"));
    }

    #[test]
    fn test_cut_by_shotlist_args() {
        let _guard = ENV_LOCK.lock().unwrap();
        let (_dir, _ffmpeg, _ffprobe) = setup_mocks();
        let input = temp_input();
        let shots = vec![
            Shot {
                start_sec: 0.0,
                end_sec: 3.0,
                label: "a".to_string(),
            },
            Shot {
                start_sec: 3.0,
                end_sec: 10.0,
                label: "b".to_string(),
            },
        ];
        let (_output, invocations) = build_cut_args(&input, &shots).unwrap();
        let flat: Vec<&String> = invocations.iter().flatten().collect();
        assert!(flat.iter().any(|a| *a == "-ss"));
        assert!(flat.iter().any(|a| *a == "concat"));
    }

    #[test]
    fn test_encode_916_args() {
        let input = temp_input();
        let (_output, args) = build_encode_916_args(&input, None).unwrap();
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.iter().any(|a| a.contains("1080:1920")));
    }

    #[test]
    fn test_extract_audio_args() {
        let input = temp_input();
        let (_output, args) = build_extract_audio_args(&input, Some("mp3".to_string())).unwrap();
        assert!(args.contains(&"libmp3lame".to_string()));
        assert!(args.contains(&"-vn".to_string()));
    }

    #[test]
    fn test_cut_by_shotlist_with_mocks() {
        let _guard = ENV_LOCK.lock().unwrap();
        let (_dir, _ffmpeg, _ffprobe) = setup_mocks();
        let ws = tempfile::tempdir().unwrap();
        std::env::set_var("EDIT_WORKSPACE", ws.path());
        let input = ws.path().join("test.mp4");
        std::fs::write(&input, b"fake").unwrap();

        let s = test_server();
        let out = s
            .cut_by_shotlist(
                input.to_string_lossy().to_string(),
                vec![
                    Shot {
                        start_sec: 0.0,
                        end_sec: 3.0,
                        label: "a".to_string(),
                    },
                    Shot {
                        start_sec: 3.0,
                        end_sec: 10.0,
                        label: "b".to_string(),
                    },
                ],
            )
            .unwrap();
        let resp: CutResponse = serde_json::from_str(&out).unwrap();
        assert_eq!(resp.segment_count, 2);
        assert!(Path::new(&resp.output_path).exists());
    }
}
