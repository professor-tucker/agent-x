# Pooled Hypervisor (prototype)

This folder contains an MVP scaffold for a pooled hypervisor / resource pooling system.

Components:

- `agent/` — Rust agent (`ph-agent`) exposing `/info` and `/run` endpoints. Currently `run` simulates execution.
- `controller/` — FastAPI controller to register agents and assign tasks.
- `docker-compose.yml` — Minimal Elasticsearch + Kibana stack for telemetry (ELC).

How to run (dev):

1. Start ELC stack (requires Docker):

```bash
cd pooled-hypervisor
docker-compose up -d
```

2. Run controller (Python):

```bash
cd pooled-hypervisor/controller
python -m venv .venv
.\.venv\Scripts\pip install -r requirements.txt   # Windows
uvicorn main:app --reload --host 0.0.0.0 --port 8080
```

3. Build and run agent (Rust):

```bash
cd pooled-hypervisor/agent
cargo run
```

4. Register agent with controller (example using httpie or curl):

```bash
curl -X POST http://localhost:8080/agents/register -H "Content-Type: application/json" -d '{"agent_id":"agent-1","url":"http://127.0.0.1:8081"}'
```

5. Assign a task to the agent:

```bash
curl -X POST http://localhost:8080/tasks/assign -H "Content-Type: application/json" -d '{"agent_id":"agent-1","task_id":"task-1","cmd":["echo","hello"]}'
```

Next steps:
- Replace simulated `run` with container runtime invocation (containerd/docker).
- Add mTLS between agent and controller and agent bootstrap flow.
- Implement workload signing (cosign) and attestation checks.
- Stream telemetry from agents into Elasticsearch.
