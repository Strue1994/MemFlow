"""Scheduler manager - Cron job orchestration"""

import uuid
from datetime import datetime
from typing import Any, Callable, Dict, List, Optional

from apscheduler.schedulers.asyncio import AsyncIOScheduler
from apscheduler.triggers.cron import CronTrigger

from scheduler.models import ScheduledJob, JobStatus
from scheduler.storage import JobStorage


class SchedulerManager:
    """
    Manages scheduled jobs with cron expressions.
    
    Features:
    - Add/pause/resume/remove jobs
    - Persistent storage in SQLite
    - Auto-recovery on restart
    
    Example:
        ```python
        async def agent_callback(task_desc: str) -> str:
            return await agent.run(task_desc)
        
        storage = JobStorage()
        manager = SchedulerManager(agent_callback, storage)
        await manager.initialize()
        
        # Add a job
        job = await manager.add_job("daily_report", "0 9 * * *", "Generate daily report")
        ```
    """

    def __init__(
        self,
        agent_loop: Callable[[str], Any],
        storage: Optional[JobStorage] = None,
    ):
        self.agent_loop = agent_loop
        self.storage = storage or JobStorage()
        self.scheduler = AsyncIOScheduler()
        self.jobs: Dict[str, ScheduledJob] = {}

    async def initialize(self) -> None:
        """Initialize scheduler and restore jobs from storage"""
        await self.storage.initialize()
        
        # Load and schedule existing jobs
        for job in await self.storage.load_all_jobs():
            self.jobs[job.id] = job
            if job.status == JobStatus.ACTIVE:
                self._add_to_scheduler(job)
        
        self.scheduler.start()

    def _add_to_scheduler(self, job: ScheduledJob) -> None:
        """Add a job to the APScheduler"""
        self.scheduler.add_job(
            func=self._execute_job_wrapper,
            trigger=CronTrigger.from_crontab(job.cron_expr),
            args=[job.id],
            id=job.id,
            name=job.name,
        )

    async def _execute_job_wrapper(self, job_id: str) -> None:
        """Wrapper to execute job with error handling"""
        await self._execute_job(job_id)

    async def _execute_job(self, job_id: str) -> None:
        """Execute a scheduled job"""
        job = self.jobs.get(job_id)
        if not job:
            return

        print(f"[Scheduler] Executing: {job.name}")
        print(f"[Scheduler] Task: {job.task_description}")

        try:
            # Call the agent loop with task description
            result = self.agent_loop(job.task_description)
            # Handle both sync and async results
            if hasattr(result, '__await__'):
                result = await result
            
            # Update last_run time
            job.last_run = datetime.now()
            await self.storage.save_job(job)
            
            print(f"[Scheduler] Completed: {result if isinstance(result, str) else str(result)[:100]}...")
        except Exception as e:
            print(f"[Scheduler] Error: {e}")

    async def add_job(
        self,
        name: str,
        cron_expr: str,
        task_description: str,
    ) -> ScheduledJob:
        """Add a new scheduled job"""
        job = ScheduledJob(
            id=str(uuid.uuid4()),
            name=name,
            cron_expr=cron_expr,
            task_description=task_description,
            status=JobStatus.ACTIVE,
            created_at=datetime.now(),
        )

        self._add_to_scheduler(job)
        self.jobs[job.id] = job
        await self.storage.save_job(job)

        return job

    async def pause_job(self, job_id: str) -> bool:
        """Pause a job"""
        if job_id not in self.jobs:
            return False

        self.scheduler.pause_job(job_id)
        self.jobs[job_id].status = JobStatus.PAUSED
        await self.storage.save_job(self.jobs[job_id])
        return True

    async def resume_job(self, job_id: str) -> bool:
        """Resume a paused job"""
        if job_id not in self.jobs:
            return False

        self.scheduler.resume_job(job_id)
        self.jobs[job_id].status = JobStatus.ACTIVE
        await self.storage.save_job(self.jobs[job_id])
        return True

    async def remove_job(self, job_id: str) -> bool:
        """Remove a job"""
        if job_id not in self.jobs:
            return False

        self.scheduler.remove_job(job_id)
        await self.storage.delete_job(job_id)
        del self.jobs[job_id]
        return True

    def list_jobs(self) -> List[ScheduledJob]:
        """List all jobs"""
        return list(self.jobs.values())

    def get_job(self, job_id: str) -> Optional[ScheduledJob]:
        """Get a job by ID"""
        return self.jobs.get(job_id)

    async def shutdown(self) -> None:
        """Shutdown the scheduler"""
        self.scheduler.shutdown()