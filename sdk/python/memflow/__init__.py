"""MemFlow Python SDK"""
from dataclasses import dataclass
from typing import Optional, Any
import httpx


@dataclass
class MemFlowConfig:
    base_url: str = "http://localhost:3300"
    api_key: Optional[str] = None


class MemFlow:
    def __init__(self, config: Optional[MemFlowConfig] = None):
        self.config = config or MemFlowConfig()
        self._client = httpx.Client(base_url=self.config.base_url)
        if self.config.api_key:
            self._client.headers["X-API-Key"] = self.config.api_key

    def _request(self, method: str, path: str, **kwargs) -> Any:
        r = self._client.request(method, path, **kwargs)
        r.raise_for_status()
        return r.json()

    @property
    def workflows(self):
        class _Workflows:
            def __init__(self, mf):
                self._mf = mf
            def execute(self, workflow_id: str, params: Optional[dict] = None):
                return self._mf._request("POST", "/execute", json={"workflow_id": workflow_id, "params": params})
            def list(self):
                return self._mf._request("GET", "/workflows")
        return _Workflows(self)

    @property
    def memory(self):
        class _Memory:
            def __init__(self, mf):
                self._mf = mf
            def search(self, query: str, k: int = 5):
                return self._mf._request("GET", f"/memories/search?q={query}&k={k}")
            def store(self, content: str, type: str = "Conversation", importance: float = 0.5):
                return self._mf._request("POST", "/memories", json={"content": content, "type": type, "importance": importance})
        return _Memory(self)

    @property
    def skills(self):
        class _Skills:
            def __init__(self, mf):
                self._mf = mf
            def list(self):
                return self._mf._request("GET", "/skills")
        return _Skills(self)

    def close(self):
        self._client.close()
