"""No inference: assert each connector really supplies the host's MCP server."""
import json
import sys

provider = sys.argv[1]
server = {"command": "/apps/pixen", "args": ["--ai-mcp-proxy"], "env": {"ART_TOKEN": "test"}}
for line in sys.stdin:
    request = json.loads(line)
    method = request.get("method")
    params = request.get("params", {})
    if method == "initialize":
        result = {"protocolVersion": 1, "agentCapabilities": {}}
    elif method == "initialized":
        continue
    elif method == "config/read":
        assert provider == "codex-app-only", provider
        result = {"config": {"mcp_servers": {"outside": {"command": "external"}}}}
    elif method == "thread/start":
        if provider == "codex-app-only":
            assert params["approvalPolicy"] == "never", params
            assert params["sandbox"] == "read-only", params
            servers = params["config"]["mcp_servers"]
            assert servers["outside"]["enabled"] is False, servers
            assert servers["art"] == {
                **server, "enabled": True,
                "enabled_tools": ["get_project_info", "apply_edits"],
                "disabled_tools": [], "default_tools_approval_mode": "prompt",
                "tools": {name: {"approval_mode": "approve"} for name in ["get_project_info", "apply_edits"]},
            }, servers
            assert params["config"]["features"]["shell_tool"] is False, params
        else:
            assert params["config"]["mcp_servers"] == {"art": server}, params
        result = {"thread": {"id": "art-session"}}
    elif method == "session/create":
        assert params["mcpServers"] == {"art": {"type": "stdio", **server}}, params
        result = {"id": "art-session", "cwd": "test-workspace", "title": None}
    elif method == "session/new":
        assert params["mcpServers"] == [{"name": "art", **server, "env": [{"name": "ART_TOKEN", "value": "test"}]}], params
        result = {"sessionId": "art-session", "models": {"availableModels": [{"modelId":"artist","name":"Artist"}]}}
    elif method == "model/list":
        result = {"data": [{"model":"artist","displayName":"Artist"}],"nextCursor":None}
    elif method == "models/list":
        result = [{"value":"artist","displayName":"Artist"}]
    elif method == "shutdown":
        print(json.dumps({"id": request["id"], "result": None}), flush=True)
        break
    else:
        raise AssertionError((provider, method, params))
    print(json.dumps({"id": request["id"], "result": result}), flush=True)
