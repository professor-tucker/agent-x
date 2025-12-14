use actix_web::{post, get, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt, CpuExt};
use std::sync::{Arc, Mutex};
use log::info;

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
        // Build docker args: docker run --rm --network none <image> <cmd...>
        let mut args: Vec<String> = vec!["run".to_string(), "--rm".to_string(), "--network".to_string(), "none".to_string(), image.clone()];
        for c in task.cmd.iter() {
            args.push(c.clone());
        }

        // Run in blocking thread to avoid blocking actix runtime.
        let task_id = task.task_id.clone();
        let output = actix_web::rt::task::spawn_blocking(move || {
            use std::process::Command;
            let mut cmd = Command::new("docker");
            for a in args.iter() {
                cmd.arg(a);
            }
            // Capture combined output
            match cmd.output() {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
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
use actix_web::{post, get, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt, CpuExt};
use std::sync::{Arc, Mutex};
use log::info;

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
    // For MVP: simulate running the task and return a canned response.
    info!("Received run request: {}", req.task_id);
    let output = format!("simulated run: {:?}", req.cmd);
    let resp = RunResponse {
        task_id: req.task_id.clone(),
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
