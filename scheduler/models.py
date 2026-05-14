"""Scheduler data models"""

from datetime import datetime
from enum import Enum
from typing import Optional

from pydantic import BaseModel, Field


class JobStatus(str, Enum):
    """Job execution status"""
    ACTIVE = "active"
    PAUSED = "paused"
    COMPLETED = "completed"


class ScheduledJob(BaseModel):
    """A scheduled job definition"""
    id: str
    name: str
    cron_expr: str
    task_description: str
    status: JobStatus = JobStatus.ACTIVE
    created_at: datetime = Field(default_factory=datetime.now)
    last_run: Optional[datetime] = None
    next_run: Optional[datetime] = None

    model_config = {"use_enum_values": True}