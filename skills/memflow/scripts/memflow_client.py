#!/usr/bin/env python3
"""
MemFlow Client - Python wrapper for MemFlow Agent Service API
"""

import os
import json
import requests
from typing import Optional, Dict, Any, Callable

MEMFLOW_URL = os.getenv("MEMFLOW_URL", "http://localhost:3000")
API_KEY = os.getenv("MEMFLOW_API_KEY", "")

headers = {"Content-Type": "application/json"}
if API_KEY:
    headers["X-API-Key"] = API_KEY


def create_workflow(n8n_json: Dict[str, Any], name: Optional[str] = None) -> Dict[str, Any]:
    """Create workflow from n8n JSON"""
    payload = {"n8n_json": n8n_json}
    if name:
        payload["name"] = name
    
    response = requests.post(f"{MEMFLOW_URL}/create_workflow", json=payload, headers=headers)
    response.raise_for_status()
    return response.json()


def create_workflow_v2(
    user_request: str,
    on_step: Optional[Callable[[Dict[str, Any]], None]] = None
) -> Dict[str, Any]:
    """Create workflow using multi-stage pipeline with SSE events"""
    response = requests.post(
        f"{MEMFLOW_URL}/create_workflow_v2_sse",
        json={"user_request": user_request},
        headers=headers,
        stream=True
    )
    response.raise_for_status()
    
    result = {}
    for line in response.iter_lines():
        if line:
            try:
                event = json.loads(line.decode("utf-8"))
                if on_step:
                    on_step(event)
                
                if event.get("status") == "completed" or event.get("step") == "completed":
                    result = event.get("payload", {})
            except json.JSONDecodeError:
                continue
    
    return result


def execute_workflow(
    workflow_id: str,
    params: Optional[Dict[str, Any]] = None
) -> Dict[str, Any]:
    """Execute a workflow"""
    payload = {"workflow_id": workflow_id}
    if params:
        payload["params"] = params
    
    response = requests.post(f"{MEMFLOW_URL}/execute", json=payload, headers=headers)
    response.raise_for_status()
    return response.json()


def list_workflows() -> list:
    """List all workflows"""
    response = requests.get(f"{MEMFLOW_URL}/workflows", headers=headers)
    response.raise_for_status()
    data = response.json()
    return data.get("workflows", [])


def get_workflow(workflow_id: str) -> Dict[str, Any]:
    """Get workflow details"""
    response = requests.get(f"{MEMFLOW_URL}/workflows/{workflow_id}", headers=headers)
    response.raise_for_status()
    return response.json()


def submit_feedback(
    pattern_id: str,
    user_request: str,
    accepted: bool,
    modifications: Optional[Dict[str, Any]] = None
) -> Dict[str, Any]:
    """Submit user feedback for pattern matching"""
    payload = {
        "pattern_id": pattern_id,
        "user_request": user_request,
        "accepted": accepted,
    }
    if modifications:
        payload["modifications"] = modifications
    
    response = requests.post(f"{MEMFLOW_URL}/feedback", json=payload, headers=headers)
    response.raise_for_status()
    return response.json()


def validate_workflow(n8n_json: Dict[str, Any]) -> list:
    """Validate workflow JSON"""
    response = requests.post(
        f"{MEMFLOW_URL}/validate",
        json={"n8n_json": n8n_json},
        headers=headers
    )
    response.raise_for_status()
    return response.json().get("issues", [])


if __name__ == "__main__":
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: memflow_client.py <command> [args]")
        print("Commands:")
        print("  list                      List all workflows")
        print("  create <description>    Create workflow from description")
        print("  execute <id>             Execute workflow by ID")
        sys.exit(1)
    
    cmd = sys.argv[1]
    
    if cmd == "list":
        for wf in list_workflows():
            print(f"  {wf.get('id')}: {wf.get('name', 'Untitled')}")
    
    elif cmd == "create":
        description = " ".join(sys.argv[2:])
        if not description:
            print("Error: Please provide a description")
            sys.exit(1)
        
        def print_step(event):
            print(f"  [{event.get('step')}] {event.get('status')}")
        
        result = create_workflow_v2(description, on_step=print_step)
        print(f"\nWorkflow created: {result.get('workflow_id')}")
    
    elif cmd == "execute":
        wf_id = sys.argv[2] if len(sys.argv) > 2 else None
        if not wf_id:
            print("Error: Please provide workflow ID")
            sys.exit(1)
        
        result = execute_workflow(wf_id)
        print(f"Execution result: {result}")
    
    else:
        print(f"Unknown command: {cmd}")