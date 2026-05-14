"""Sandbox Execution Environment with E2B integration"""

import os
from typing import Any, Dict, Optional
from dataclasses import dataclass


@dataclass
class ExecutionResult:
    """Result of sandbox execution"""
    stdout: str = ""
    stderr: str = ""
    error: Optional[str] = None
    logs: list[str] = None
    
    def __post_init__(self):
        if self.logs is None:
            self.logs = []


class Sandbox:
    """
    Provides isolated environment for code execution.
    
    Supports multiple backends:
    - E2B (e2b.dev) - Cloud-based sandbox
    - Docker - Local container (TODO)
    - Local exec - For trusted code only (TODO)
    """
    
    def __init__(self, backend: str = "e2b"):
        self.backend = backend
        self._e2b_api_key = os.getenv("E2B_API_KEY")
        
    async def execute_code(
        self,
        code: str,
        language: str = "python",
    ) -> ExecutionResult:
        """
        Execute code in isolated sandbox.
        
        Args:
            code: Source code to execute
            language: Programming language (python, javascript, etc.)
            
        Returns:
            ExecutionResult with stdout, stderr, and error fields
        """
        if language != "python":
            return ExecutionResult(
                error=f"Language '{language}' not supported. Supported: python"
            )
            
        if self.backend == "e2b":
            return await self._execute_e2b(code)
        else:
            return await self._execute_local(code)

    async def _execute_e2b(self, code: str) -> ExecutionResult:
        """
        Execute using E2B cloud sandbox.
        
        Uses the e2b-code-interpreter SDK for secure code execution.
        """
        if not self._e2b_api_key:
            return ExecutionResult(
                error="E2B_API_KEY not configured. Set environment variable or provide API key."
            )
        
        try:
            from e2b_code_interpreter import Sandbox as E2BSandbox
            
            with E2BSandbox(api_key=self._e2b_api_key) as e2b:
                result = e2b.exec(code)
                
                return ExecutionResult(
                    stdout=result.stdout or "",
                    stderr=result.stderr or "",
                    error=None if result.error is None else str(result.error),
                    logs=result.logs if hasattr(result, 'logs') else []
                )
        except ImportError:
            return ExecutionResult(
                error="e2b-code-interpreter not installed. Run: pip install e2b-code-interpreter"
            )
        except Exception as e:
            return ExecutionResult(error=f"E2B execution failed: {e}")

    async def _execute_local(self, code: str) -> ExecutionResult:
        """
        Execute locally - USE WITH CAUTION.
        
        Only use for trusted code in development.
        """
        # Don't allow actual execution without proper sandboxing
        return ExecutionResult(
            error="[Sandbox] Local execution not implemented - use E2B backend"
        )

    async def execute_javascript(self, code: str) -> ExecutionResult:
        """Execute JavaScript code via E2B"""
        if self.backend == "e2b":
            try:
                from e2b_code_interpreter import Sandbox as E2BSandbox
                
                with E2BSandbox(api_key=self._e2b_api_key) as e2b:
                    result = e2b.exec(code, language="javascript")
                    return ExecutionResult(
                        stdout=result.stdout or "",
                        stderr=result.stderr or "",
                        error=None if result.error is None else str(result.error)
                    )
            except Exception as e:
                return ExecutionResult(error=f"JavaScript execution failed: {e}")
        return ExecutionResult(error="JavaScript not supported with current backend")
