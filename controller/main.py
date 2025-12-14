from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import httpx
import asyncio

app = FastAPI()

class AgentInfo(BaseModel):
    agent_id: str
    url: str

class RunTask(BaseModel):
    agent_id: str
    task_id: str
    cmd: list[str]

# In-memory registry (MVP)
AGENTS: dict[str, str] = {}


@app.post("/agents/register")
async def register_agent(a: AgentInfo):
    AGENTS[a.agent_id] = a.url
    return {"status": "ok", "agent_id": a.agent_id}


@app.get("/agents")
async def list_agents():
    return [{"agent_id": k, "url": v} for k, v in AGENTS.items()]


@app.post("/tasks/assign")
async def assign_task(t: RunTask):
    url = AGENTS.get(t.agent_id)
    if not url:
        raise HTTPException(status_code=404, detail="agent not found")
    run_url = f"{url.rstrip('/')}/run"
    payload = {"task_id": t.task_id, "cmd": t.cmd}
    async with httpx.AsyncClient(timeout=30.0) as client:
        resp = await client.post(run_url, json=payload)
        resp.raise_for_status()
        return resp.json()


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8080)
