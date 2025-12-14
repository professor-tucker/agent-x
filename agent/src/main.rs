use actix_web::{post, get, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt, CpuExt};
use std::sync::{Arc, Mutex};
use log::info;
use std::process::Command;
use std::time::Duration;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;

#[derive(Serialize)]
struct Info {
    hostname: String,
    cpu_usage_percent: f32,
    total_memory_kb: u64,
    used_memory_kb: u64,
}

#[derive(Deserialize)]
struct RunRequest {
    task_id: String,
    // Optional container image to run (if omitted, falls back to simulated run)
    image: Option<String>,
    // Command and arguments to pass to the container or simulate
    cmd: Vec<String>,
}

#[derive(Serialize)]
struct RunResponse {
    task_id: String,
    status: String,
    output: String,
}

struct AppState {
    sys: Mutex<System>,
}

fn send_telemetry(elastic: &str, payload: &serde_json::Value) {
    if elastic.is_empty() { return }
    let client = match Client::builder().timeout(Duration::from_secs(5)).build() {
        Ok(c) => c,
        Err(_) => return,
    };
    let idx = "ph-agent-telemetry";
    let url = format!("{}/{}/_doc", elastic.trim_end_matches('/'), idx);
    // retry with simple exponential backoff
    for attempt in 0..3 {
        let res = client.post(&url)
            .header(CONTENT_TYPE, "application/json")
            .json(payload)
            .send();
        match res {
            Ok(r) if r.status().is_success() => return,
            _ => {
                let backoff = std::time::Duration::from_millis(100u64 * (1 << attempt));
                std::thread::sleep(backoff);
            }
        }
    }
}

#[get("/info")]
async fn info(data: web::Data<Arc<AppState>>) -> impl Responder {
    let mut sys = data.sys.lock().unwrap();
    sys.refresh_cpu();
    sys.refresh_memory();
    let hostname = sys.host_name().unwrap_or_else(|| "unknown".to_string());
    let cpu = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / (sys.cpus().len() as f32);
    let info = Info {
        hostname,
        cpu_usage_percent: cpu,
        total_memory_kb: sys.total_memory(),
        used_memory_kb: sys.used_memory(),
    };
    HttpResponse::Ok().json(info)
}

#[post("/run")]
async fn run_task(req: web::Json<RunRequest>) -> impl Responder {
    info!("Received run request: {}", req.task_id);

    let task = req.into_inner();
    // If an image is provided, attempt to run it using `docker run` (MVP).
    if let Some(image) = task.image {
        // Optional: verify image signature if COSIGN_KEY is provided
        if let Ok(cosign_key) = std::env::var("COSIGN_PUBLIC_KEY") {
            // best-effort: run `cosign verify --key <key> <image>`
            let verify_res = Command::new("cosign")
                .arg("verify")
                .arg("--key")
                .arg(cosign_key)
                .arg(&image)
                .output();
            if let Ok(o) = verify_res {
                if !o.status.success() {
                    let resp = RunResponse { task_id: task.task_id.clone(), status: "rejected".to_string(), output: "image signature verification failed".to_string() };
                    return HttpResponse::Forbidden().json(resp);
                }
            }
        }

        // Hardened docker run arguments
        let mem = std::env::var("DOCKER_MEMORY").unwrap_or_else(|_| "256m".to_string());
        let cpus = std::env::var("DOCKER_CPUS").unwrap_or_else(|_| "0.5".to_string());
        let user = std::env::var("DOCKER_USER").unwrap_or_else(|_| "1000:1000".to_string());

        let mut args: Vec<String> = vec![
            "run".to_string(),
            "--rm".to_string(),
            "--network".to_string(),
            "none".to_string(),
            "--read-only".to_string(),
            "--cap-drop".to_string(),
            "ALL".to_string(),
            "--security-opt".to_string(),
            "no-new-privileges:true".to_string(),
            "--memory".to_string(),
            mem,
            "--cpus".to_string(),
            cpus,
            "--user".to_string(),
            user,
            image.clone(),
        ];
        for c in task.cmd.iter() {
            args.push(c.clone());
        }

        // Run in blocking thread to avoid blocking actix runtime.
        let task_id = task.task_id.clone();
        let elastic = std::env::var("ELASTIC_URL").unwrap_or_default();
        let output = actix_web::rt::task::spawn_blocking(move || {
            let mut cmd = Command::new("docker");
            for a in args.iter() {
                cmd.arg(a);
            }
            match cmd.output() {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
                    // send telemetry
                    if !elastic.is_empty() {
                        let payload = serde_json::json!({"task_id": task_id, "status": "completed", "output_snip": &combined[..std::cmp::min(512, combined.len())]});
                        send_telemetry(&elastic, &payload);
                    }
                    Ok((task_id, combined))
                }
                Err(e) => Ok((task_id, format!("failed to execute docker: {}", e))),
            }
        })
        .await;

        match output {
            Ok(Ok((tid, out))) => {
                let resp = RunResponse {
                    task_id: tid,
                    status: "completed".to_string(),
                    output: out,
                };
                return HttpResponse::Ok().json(resp);
            }
            _ => {
                let resp = RunResponse {
                    task_id: task.task_id.clone(),
                    status: "error".to_string(),
                    output: "docker run failed or was interrupted".to_string(),
                };
                return HttpResponse::InternalServerError().json(resp);
            }
        }
    }

    // Fallback: simulate running the task and return a canned response.
    let output = format!("simulated run: {:?}", task.cmd);
    let resp = RunResponse {
        task_id: task.task_id.clone(),
        status: "completed".to_string(),
        output,
    };
    HttpResponse::Ok().json(resp)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let sys = System::new_all();
    let state = Arc::new(AppState { sys: Mutex::new(sys) });
    let bind = std::env::var("PH_AGENT_BIND").unwrap_or_else(|_| "0.0.0.0:8081".to_string());
    info!("Starting ph-agent on {}", bind);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(info)
            .service(run_task)
    })
    .bind(bind)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_noop_on_empty_url() {
        // should not panic when elastic URL is empty
        let payload = serde_json::json!({"k":"v"});
        send_telemetry("", &payload);
    }

    #[test]
    fn serialize_run_response() {
        let r = RunResponse { task_id: "t1".into(), status: "ok".into(), output: "out".into() };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("t1"));
    }
}
