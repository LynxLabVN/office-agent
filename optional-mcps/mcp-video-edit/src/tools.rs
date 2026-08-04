use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::engine::{self};
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
    engine: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OutputPathResponse {
    output_path: String,
    engine: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CutResponse {
    output_path: String,
    segment_count: usize,
    engine: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EncodeResponse {
    output_path: String,
    width: u32,
    height: u32,
    engine: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExtractAudioResponse {
    output_path: String,
    engine: String,
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

#[tool(tool_box)]
impl VideoEditServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-video-edit".to_string(),
            engine: "ffmpeg".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "cut_by_shotlist", description = "Cut a video into segments by shot list and concat them")]
    pub fn cut_by_shotlist(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] shots: Vec<Shot>,
        #[tool(param)] engine: Option<String>,
    ) -> Result<String, String> {
        let eng = engine::resolve_engine(engine.as_deref());
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let outcome = eng.cut(&input, &shots)?;
        serde_json::to_string(&CutResponse {
            output_path: outcome.output_path.to_string_lossy().to_string(),
            segment_count: outcome.segment_count,
            engine: eng.name().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "burn_captions", description = "Burn subtitles or auto-transcribe captions into a video")]
    pub fn burn_captions(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] captions: Option<Vec<Caption>>,
        #[tool(param)] style: Option<CaptionStyle>,
        #[tool(param)] engine: Option<String>,
    ) -> Result<String, String> {
        let eng = engine::resolve_engine(engine.as_deref());
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let output = eng.burn_captions(&input, captions, style)?;
        serde_json::to_string(&OutputPathResponse {
            output_path: output.to_string_lossy().to_string(),
            engine: eng.name().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "overlay_text", description = "Overlay text on a video using drawtext")]
    pub fn overlay_text(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] overlays: Vec<Overlay>,
        #[tool(param)] engine: Option<String>,
    ) -> Result<String, String> {
        let eng = engine::resolve_engine(engine.as_deref());
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let output = eng.overlay(&input, overlays)?;
        serde_json::to_string(&OutputPathResponse {
            output_path: output.to_string_lossy().to_string(),
            engine: eng.name().to_string(),
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
        #[tool(param)] engine: Option<String>,
    ) -> Result<String, String> {
        let eng = engine::resolve_engine(engine.as_deref());
        let input = input_path(&input)?;
        let music = input_path(&music)?;
        ensure_output_dir()?;
        let output = eng.add_music(&input, &music, level_db, duck)?;
        serde_json::to_string(&OutputPathResponse {
            output_path: output.to_string_lossy().to_string(),
            engine: eng.name().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "encode_916", description = "Encode a video to 9:16 vertical 1080x1920")]
    pub fn encode_916(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] quality: Option<String>,
        #[tool(param)] engine: Option<String>,
    ) -> Result<String, String> {
        let eng = engine::resolve_engine(engine.as_deref());
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let outcome = eng.render(&input, quality)?;
        serde_json::to_string(&EncodeResponse {
            output_path: outcome.output_path.to_string_lossy().to_string(),
            width: outcome.width,
            height: outcome.height,
            engine: eng.name().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "concat", description = "Concatenate multiple videos")]
    pub fn concat(
        &self,
        #[tool(param)] inputs: Vec<String>,
        #[tool(param)] transition: Option<String>,
        #[tool(param)] engine: Option<String>,
    ) -> Result<String, String> {
        let eng = engine::resolve_engine(engine.as_deref());
        let inputs: Result<Vec<_>, _> = inputs.iter().map(|p| input_path(p)).collect();
        let inputs = inputs?;
        ensure_output_dir()?;
        let output = eng.concat(&inputs, transition)?;
        serde_json::to_string(&OutputPathResponse {
            output_path: output.to_string_lossy().to_string(),
            engine: eng.name().to_string(),
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "extract_audio", description = "Extract audio track from a video")]
    pub fn extract_audio(
        &self,
        #[tool(param)] input: String,
        #[tool(param)] format: Option<String>,
        #[tool(param)] engine: Option<String>,
    ) -> Result<String, String> {
        let eng = engine::resolve_engine(engine.as_deref());
        let input = input_path(&input)?;
        ensure_output_dir()?;
        let output = eng.extract_audio(&input, format)?;
        serde_json::to_string(&ExtractAudioResponse {
            output_path: output.to_string_lossy().to_string(),
            engine: eng.name().to_string(),
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
    use crate::engine::EditEngine;
    use std::io::Write;
    use std::path::Path;
    use std::sync::Mutex;

    // Env vars are process-global; serialize tests that mutate them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn mock_bin(dir: &Path, name: &str, script: &str) -> PathBuf {
        // Unix: write an executable #!/bin/sh mock.
        #[cfg(unix)]
        {
            let path = dir.join(name);
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(script.as_bytes()).unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).unwrap();
            path
        }
        // Windows: a #!/bin/sh script is not executable via std::process::Command,
        // so emit an equivalent .bat instead. The ffprobe mock prints a duration
        // float; the ffmpeg mock creates the output file (last arg without a `-`).
        #[cfg(windows)]
        {
            let _ = script;
            let path = dir.join(format!("{name}.bat"));
            let bat: &str = match name {
                "ffprobe" => "@echo off\r\necho 10.0\r\n",
                _ => {
                    "@echo off\r\n\
                     setlocal enabledelayedexpansion\r\n\
                     set \"out=\"\r\n\
                     :parse\r\n\
                     if \"%~1\"==\"\" goto exec\r\n\
                     set \"a=%~1\"\r\n\
                     if \"!a:~0,1!\"==\"-\" ( shift & goto parse )\r\n\
                     set \"out=%~1\"\r\n\
                     shift\r\n\
                     goto parse\r\n\
                     :exec\r\n\
                     if defined out (\r\n\
                       for %%F in (\"%out%\") do ( if not exist \"%%~dpF\" mkdir \"%%~dpF\" )\r\n\
                       echo mock> \"%out%\"\r\n\
                     )\r\n\
                     endlocal\r\n"
                }
            };
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(bat.as_bytes()).unwrap();
            path
        }
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
        assert!(out.contains("\"engine\":\"ffmpeg\""));
    }

    #[test]
    fn test_resolve_engine_default() {
        assert_eq!(engine::resolve_engine(None).name(), "ffmpeg");
        assert_eq!(engine::resolve_engine(Some("ffmpeg")).name(), "ffmpeg");
        assert_eq!(engine::resolve_engine(Some("davinci")).name(), "davinci");
        // Unknown engine falls back to ffmpeg (lenient default).
        assert_eq!(engine::resolve_engine(Some("bogus")).name(), "ffmpeg");
    }

    #[test]
    fn test_engine_param_ffmpeg_with_mocks() {
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
                Some("ffmpeg".to_string()),
            )
            .unwrap();
        let resp: CutResponse = serde_json::from_str(&out).unwrap();
        assert_eq!(resp.segment_count, 2);
        assert_eq!(resp.engine, "ffmpeg");
        assert!(Path::new(&resp.output_path).exists());
    }

    #[test]
    fn test_engine_param_davinci_no_bridge_errors() {
        // Davinci engine shells out to the python bridge. With DAVINCI_BRIDGE set
        // to a missing path, bridge resolution fails before any subprocess spawn.
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.mp4");
        std::fs::write(&input, b"fake").unwrap();
        let saved_bridge = std::env::var("DAVINCI_BRIDGE").ok();
        std::env::set_var("DAVINCI_BRIDGE", "/no/such/davinci_bridge.py");
        let s = test_server();
        let err = s
            .cut_by_shotlist(
                input.to_string_lossy().to_string(),
                vec![Shot {
                    start_sec: 0.0,
                    end_sec: 1.0,
                    label: "a".to_string(),
                }],
                Some("davinci".to_string()),
            )
            .unwrap_err();
        assert!(
            err.contains("DAVINCI_BRIDGE"),
            "expected bridge-not-found error, got: {err}"
        );
        match saved_bridge {
            Some(v) => std::env::set_var("DAVINCI_BRIDGE", v),
            None => std::env::remove_var("DAVINCI_BRIDGE"),
        }
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
                None, // no engine param = ffmpeg (backward compat)
            )
            .unwrap();
        let resp: CutResponse = serde_json::from_str(&out).unwrap();
        assert_eq!(resp.segment_count, 2);
        assert_eq!(resp.engine, "ffmpeg");
        assert!(Path::new(&resp.output_path).exists());
    }
}