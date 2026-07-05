use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool, ServerHandler,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct HrDataServer {
    pub db: Arc<Mutex<Connection>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthResponse {
    ok: bool,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JobIdResponse {
    job_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApplicationIdResponse {
    application_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OkResponse {
    ok: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct StageUpdateResponse {
    ok: bool,
    previous_stage: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PipelineStats {
    applied: i64,
    screened: i64,
    shortlist: i64,
    interview: i64,
    offer: i64,
    hired: i64,
    rejected: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct JobRow {
    id: String,
    title: String,
    dept: String,
    exp_level: String,
    status: String,
    created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApplicationRow {
    id: String,
    candidate_id: String,
    stage: String,
    cv_score: Option<f64>,
    score_override: Option<f64>,
    applied_at: String,
}

fn new_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

fn comma_join(items: &[String]) -> String {
    items.join(",")
}

fn comma_split(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn allowed_next_stages(current: &str) -> Vec<&'static str> {
    match current {
        "applied" => vec!["screened", "rejected"],
        "screened" => vec!["shortlist", "rejected"],
        "shortlist" => vec!["interview", "rejected"],
        "interview" => vec!["offer", "rejected"],
        "offer" => vec!["hired", "rejected"],
        "hired" => vec![],
        "rejected" => vec![],
        _ => vec!["applied"],
    }
}

#[tool(tool_box)]
impl HrDataServer {
    #[tool(name = "health", description = "Return server health status")]
    pub fn health(&self) -> String {
        serde_json::to_string(&HealthResponse {
            ok: true,
            name: "mcp-hr-data".to_string(),
        })
        .unwrap_or_else(|_| r#"{"ok":false}"#.to_string())
    }

    #[tool(name = "create_job", description = "Create a job requisition and return its id")]
    #[allow(clippy::too_many_arguments)]
    pub fn create_job(
        &self,
        #[tool(param)] title: String,
        #[tool(param)] dept: String,
        #[tool(param)] jd_markdown: String,
        #[tool(param)] skills_required: Vec<String>,
        #[tool(param)] skills_nice: Option<Vec<String>>,
        #[tool(param)] exp_level: String,
        #[tool(param)] salary_min_vnd: Option<i64>,
        #[tool(param)] salary_max_vnd: Option<i64>,
        #[tool(param)] location: String,
        #[tool(param)] benefits: Option<String>,
    ) -> Result<String, String> {
        let job_id = new_id("job");
        let skills_nice = skills_nice.unwrap_or_default();
        let benefits = benefits.unwrap_or_default();

        let conn = self.db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO jobs (id, title, dept, jd_markdown, skills_required, skills_nice, \
             exp_level, salary_min_vnd, salary_max_vnd, location, benefits) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &job_id,
                title,
                dept,
                jd_markdown,
                comma_join(&skills_required),
                comma_join(&skills_nice),
                exp_level,
                salary_min_vnd,
                salary_max_vnd,
                location,
                benefits,
            ],
        )
        .map_err(|e| e.to_string())?;

        crate::audit::log(
            "mcp_call",
            &format!("mcp-hr-data.create_job:{}", job_id),
            None,
            Some(serde_json::json!({"title": title, "dept": dept, "exp_level": exp_level})),
        );

        serde_json::to_string(&JobIdResponse { job_id }).map_err(|e| e.to_string())
    }

    #[tool(name = "list_jobs", description = "List jobs with optional status filter")]
    pub fn list_jobs(
        &self,
        #[tool(param)] status: Option<String>,
    ) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, dept, exp_level, status, created_at FROM jobs WHERE 1=1 \
                 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;

        let rows = if let Some(status) = status {
            stmt.query_map([&status], |row| {
                Ok(JobRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    dept: row.get(2)?,
                    exp_level: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
        } else {
            stmt.query_map([], |row| {
                Ok(JobRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    dept: row.get(2)?,
                    exp_level: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
        };

        serde_json::to_string(&rows).map_err(|e| e.to_string())
    }

    #[tool(name = "get_job", description = "Get full job row by id")]
    pub fn get_job(&self, #[tool(param)] job_id: String) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, dept, jd_markdown, skills_required, skills_nice, exp_level, \
                 salary_min_vnd, salary_max_vnd, location, benefits, status, created_at, updated_at \
                 FROM jobs WHERE id = ?",
            )
            .map_err(|e| e.to_string())?;

        let row = stmt
            .query_row([&job_id], |row| {
                let skills_required: String = row.get(4)?;
                let skills_nice: String = row.get(5)?;
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "dept": row.get::<_, String>(2)?,
                    "jd_markdown": row.get::<_, String>(3)?,
                    "skills_required": comma_split(&skills_required),
                    "skills_nice": comma_split(&skills_nice),
                    "exp_level": row.get::<_, String>(6)?,
                    "salary_min_vnd": row.get::<_, Option<i64>>(7)?,
                    "salary_max_vnd": row.get::<_, Option<i64>>(8)?,
                    "location": row.get::<_, String>(9)?,
                    "benefits": row.get::<_, String>(10)?,
                    "status": row.get::<_, String>(11)?,
                    "created_at": row.get::<_, String>(12)?,
                    "updated_at": row.get::<_, String>(13)?,
                }))
            })
            .map_err(|e| e.to_string())?;

        serde_json::to_string(&row).map_err(|e| e.to_string())
    }

    #[tool(name = "save_application", description = "Save a candidate application for a job")]
    pub fn save_application(
        &self,
        #[tool(param)] job_id: String,
        #[tool(param)] candidate_id: String,
        #[tool(param)] source: String,
    ) -> Result<String, String> {
        let application_id = new_id("app");

        let mut conn = self.db.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        // Ensure candidate exists; if not, create a placeholder so foreign-key
        // constraints and downstream lookups remain valid. The agent should
        // update the candidate record later via get_candidate + external flow.
        let candidate_exists: bool = tx
            .query_row(
                "SELECT 1 FROM candidates WHERE id = ?",
                [&candidate_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !candidate_exists {
            tx.execute(
                "INSERT INTO candidates (id, full_name) VALUES (?1, ?2)",
                params![&candidate_id, format!("Candidate {}", &candidate_id)],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.execute(
            "INSERT INTO applications (id, job_id, candidate_id, source) \
             VALUES (?1, ?2, ?3, ?4)",
            params![&application_id, job_id, candidate_id, source],
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;

        crate::audit::log(
            "mcp_call",
            &format!("mcp-hr-data.save_application:{}", application_id),
            None,
            Some(serde_json::json!({"job_id": job_id, "candidate_id": candidate_id, "source": source})),
        );

        serde_json::to_string(&ApplicationIdResponse { application_id }).map_err(|e| e.to_string())
    }

    #[tool(name = "list_applications", description = "List applications for a job with optional stage filter")]
    pub fn list_applications(
        &self,
        #[tool(param)] job_id: String,
        #[tool(param)] stage: Option<String>,
    ) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;

        let (sql, params_refs): (String, Vec<&dyn rusqlite::ToSql>) = if let Some(stage) = &stage {
            (
                "SELECT id, candidate_id, stage, cv_score, score_override, applied_at FROM applications \
                 WHERE job_id = ? AND stage = ? ORDER BY applied_at DESC"
                    .to_string(),
                vec![&job_id as &dyn rusqlite::ToSql, stage as &dyn rusqlite::ToSql],
            )
        } else {
            (
                "SELECT id, candidate_id, stage, cv_score, score_override, applied_at FROM applications \
                 WHERE job_id = ? ORDER BY applied_at DESC"
                    .to_string(),
                vec![&job_id as &dyn rusqlite::ToSql],
            )
        };

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(ApplicationRow {
                    id: row.get(0)?,
                    candidate_id: row.get(1)?,
                    stage: row.get(2)?,
                    cv_score: row.get(3)?,
                    score_override: row.get(4)?,
                    applied_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        serde_json::to_string(&rows).map_err(|e| e.to_string())
    }

    #[tool(name = "get_candidate", description = "Get candidate row plus parsed_json")]
    pub fn get_candidate(
        &self,
        #[tool(param)] candidate_id: String,
    ) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let row = conn
            .query_row(
                "SELECT id, full_name, email, phone, zalo_uid, cv_file_path, portfolio_urls, \
                 parsed_json, created_at FROM candidates WHERE id = ?",
                [&candidate_id],
                |row| {
                    let portfolio_urls: String = row.get(6)?;
                    let parsed_json: Option<String> = row.get(7)?;
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "full_name": row.get::<_, String>(1)?,
                        "email": row.get::<_, Option<String>>(2)?,
                        "phone": row.get::<_, Option<String>>(3)?,
                        "zalo_uid": row.get::<_, Option<String>>(4)?,
                        "cv_file_path": row.get::<_, Option<String>>(5)?,
                        "portfolio_urls": portfolio_urls.lines().map(|s| s.to_string()).collect::<Vec<_>>(),
                        "parsed_json": parsed_json.as_deref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
                        "created_at": row.get::<_, String>(8)?,
                    }))
                },
            )
            .map_err(|e| e.to_string())?;

        serde_json::to_string(&row).map_err(|e| e.to_string())
    }

    #[tool(name = "save_interview_note", description = "Save notes and decision for an interview")]
    pub fn save_interview_note(
        &self,
        #[tool(param)] interview_id: String,
        #[tool(param)] notes_markdown: String,
        #[tool(param)] decision: String,
    ) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE interviews SET notes_markdown = ?1, decision = ?2, updated_at = datetime('now') \
             WHERE id = ?3",
            params![notes_markdown, decision, interview_id],
        )
        .map_err(|e| e.to_string())?;

        serde_json::to_string(&OkResponse { ok: true }).map_err(|e| e.to_string())
    }

    #[tool(name = "update_stage", description = "Move an application to a new stage")]
    pub fn update_stage(
        &self,
        #[tool(param)] application_id: String,
        #[tool(param)] new_stage: String,
    ) -> Result<String, String> {
        let mut conn = self.db.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let current_stage: String = tx
            .query_row(
                "SELECT stage FROM applications WHERE id = ?",
                [&application_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let allowed = allowed_next_stages(&current_stage);
        if !allowed.contains(&new_stage.as_str()) {
            return Err(format!(
                "disallowed transition from '{}' to '{}'. allowed-next: {:?}",
                current_stage, new_stage, allowed
            ));
        }

        tx.execute(
            "UPDATE applications SET stage = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![new_stage, application_id],
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;

        crate::audit::log(
            "state_transition",
            &application_id,
            Some(serde_json::json!({"stage": current_stage})),
            Some(serde_json::json!({"stage": new_stage})),
        );

        serde_json::to_string(&StageUpdateResponse {
            ok: true,
            previous_stage: current_stage,
        })
        .map_err(|e| e.to_string())
    }

    #[tool(name = "set_score_override", description = "Set a human-reviewed score override for an application")]
    pub fn set_score_override(
        &self,
        #[tool(param)] application_id: String,
        #[tool(param)] score_override: f64,
        #[tool(param)] reason: Option<String>,
    ) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let previous_score: Option<f64> = conn
            .query_row(
                "SELECT cv_score FROM applications WHERE id = ?",
                [&application_id],
                |row| row.get(0),
            )
            .ok();

        let reason_text = reason.clone().unwrap_or_default();
        conn.execute(
            "UPDATE applications SET score_override = ?1, score_breakdown = json_set(COALESCE(score_breakdown, '{}'), '$.override_reason', ?2), updated_at = datetime('now') WHERE id = ?3",
            params![score_override, reason_text, application_id],
        )
        .map_err(|e| e.to_string())?;

        crate::audit::log(
            "human_decision",
            &application_id,
            Some(serde_json::json!({"cv_score": previous_score})),
            Some(serde_json::json!({"score_override": score_override, "reason": reason})),
        );

        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "application_id": application_id,
            "previous_cv_score": previous_score,
            "score_override": score_override,
            "reason": reason_text,
        }))
        .map_err(|e| e.to_string())
    }

    #[tool(name = "get_pipeline_stats", description = "Get stage counts for a job")]
    pub fn get_pipeline_stats(
        &self,
        #[tool(param)] job_id: String,
    ) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut stats = PipelineStats {
            applied: 0,
            screened: 0,
            shortlist: 0,
            interview: 0,
            offer: 0,
            hired: 0,
            rejected: 0,
        };

        let mut stmt = conn
            .prepare("SELECT stage, COUNT(*) FROM applications WHERE job_id = ? GROUP BY stage")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([&job_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        for (stage, count) in rows {
            match stage.as_str() {
                "applied" => stats.applied = count,
                "screened" => stats.screened = count,
                "shortlist" => stats.shortlist = count,
                "interview" => stats.interview = count,
                "offer" => stats.offer = count,
                "hired" => stats.hired = count,
                "rejected" => stats.rejected = count,
                _ => {}
            }
        }

        serde_json::to_string(&stats).map_err(|e| e.to_string())
    }
}

impl ServerHandler for HrDataServer {
    rmcp::tool_box!(@derive);

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "mcp-hr-data".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("HR data MCP server".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_server() -> HrDataServer {
        let db = db::open_memory().expect("open memory db");
        HrDataServer {
            db: Arc::new(Mutex::new(db)),
        }
    }

    #[test]
    fn test_health() {
        let s = test_server();
        let out = s.health();
        assert!(out.contains("\"ok\":true"));
        assert!(out.contains("mcp-hr-data"));
    }

    #[test]
    fn test_create_job_and_get_job() {
        let s = test_server();
        let out = s
            .create_job(
                "Audio Engineer".to_string(),
                "AV".to_string(),
                "Mix live sound.".to_string(),
                vec!["audio".to_string(), "ffmpeg".to_string()],
                None,
                "mid".to_string(),
                None,
                None,
                "HCM".to_string(),
                None,
            )
            .unwrap();
        let resp: JobIdResponse = serde_json::from_str(&out).unwrap();
        assert!(resp.job_id.starts_with("job-"));

        let job_json = s.get_job(resp.job_id).unwrap();
        let job: serde_json::Value = serde_json::from_str(&job_json).unwrap();
        assert_eq!(job["title"], "Audio Engineer");
        assert_eq!(job["skills_required"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_save_application_and_pipeline_stats() {
        let s = test_server();
        let job = s
            .create_job(
                "Audio Engineer".to_string(),
                "AV".to_string(),
                "Mix live sound.".to_string(),
                vec!["audio".to_string()],
                None,
                "mid".to_string(),
                None,
                None,
                "HCM".to_string(),
                None,
            )
            .unwrap();
        let job_id: JobIdResponse = serde_json::from_str(&job).unwrap();

        let app = s
            .save_application(
                job_id.job_id.clone(),
                "cand-001".to_string(),
                "zalo_oa".to_string(),
            )
            .unwrap();
        let app: ApplicationIdResponse = serde_json::from_str(&app).unwrap();

        // legal progression
        let upd = s
            .update_stage(app.application_id.clone(), "screened".to_string())
            .unwrap();
        assert!(upd.contains("\"ok\":true"));

        let stats = s.get_pipeline_stats(job_id.job_id).unwrap();
        let stats: PipelineStats = serde_json::from_str(&stats).unwrap();
        assert_eq!(stats.applied, 0);
        assert_eq!(stats.screened, 1);
    }

    #[test]
    fn test_illegal_stage_transition() {
        let s = test_server();
        let job = s
            .create_job(
                "Audio Engineer".to_string(),
                "AV".to_string(),
                "Mix live sound.".to_string(),
                vec!["audio".to_string()],
                None,
                "mid".to_string(),
                None,
                None,
                "HCM".to_string(),
                None,
            )
            .unwrap();
        let job_id: JobIdResponse = serde_json::from_str(&job).unwrap();

        let app = s
            .save_application(job_id.job_id, "cand-002".to_string(), "email".to_string())
            .unwrap();
        let app: ApplicationIdResponse = serde_json::from_str(&app).unwrap();

        let err = s
            .update_stage(app.application_id, "hired".to_string())
            .unwrap_err();
        assert!(err.contains("disallowed transition"));
        assert!(err.contains("allowed-next"));
    }

    #[test]
    fn test_set_score_override() {
        let s = test_server();
        let job = s
            .create_job(
                "Audio Engineer".to_string(),
                "AV".to_string(),
                "Mix live sound.".to_string(),
                vec!["audio".to_string()],
                None,
                "mid".to_string(),
                None,
                None,
                "HCM".to_string(),
                None,
            )
            .unwrap();
        let job_id: JobIdResponse = serde_json::from_str(&job).unwrap();

        let app = s
            .save_application(job_id.job_id, "cand-003".to_string(), "email".to_string())
            .unwrap();
        let app: ApplicationIdResponse = serde_json::from_str(&app).unwrap();

        let out = s
            .set_score_override(app.application_id.clone(), 92.5, Some("Strong portfolio".to_string()))
            .unwrap();
        let resp: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(resp["ok"].as_bool().unwrap());
        assert_eq!(resp["score_override"].as_f64().unwrap(), 92.5);
    }
}
