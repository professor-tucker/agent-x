import pytest
import httpx
import respx
from main import app, AGENTS


@pytest.mark.asyncio
async def test_assign_task_calls_agent(respx_mock):
    AGENTS.clear()
    AGENTS['agent1'] = 'http://agent.local'
    route = respx_mock.post("http://agent.local/run").mock(return_value=httpx.Response(200, json={"task_id":"t1","status":"completed"}))

    async with httpx.AsyncClient(app=app, base_url="http://test") as client:
        resp = await client.post("/tasks/assign", json={"agent_id":"agent1","task_id":"t1","cmd":["echo"]})
        assert resp.status_code == 200
        body = resp.json()
        assert body["status"] == "completed"
        assert route.called
