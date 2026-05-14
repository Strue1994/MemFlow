"""Job storage with SQLite persistence"""

import aiosqlite
from datetime import datetime
from pathlib import Path
from typing import List, Optional

from scheduler.models import ScheduledJob, JobStatus


class JobStorage:
    """SQLite-based job storage"""

    def __init__(self, db_path: str = "data/scheduler.db"):
        self.db_path = db_path
        Path(db_path).parent.mkdir(parents=True, exist_ok=True)

    async def initialize(self) -> None:
        """Create tables if not exist"""
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute("""
                CREATE TABLE IF NOT EXISTS jobs (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    cron_expr TEXT NOT NULL,
                    task_description TEXT NOT NULL,
                    status TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    last_run TEXT,
                    next_run TEXT
                )
            """)
            await db.commit()

    async def save_job(self, job: ScheduledJob) -> None:
        """Save or update a job"""
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute("""
                INSERT OR REPLACE INTO jobs 
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                job.id,
                job.name,
                job.cron_expr,
                job.task_description,
                job.status.value,
                job.created_at.isoformat(),
                job.last_run.isoformat() if job.last_run else None,
                job.next_run.isoformat() if job.next_run else None,
            ))
            await db.commit()

    async def load_all_jobs(self) -> List[ScheduledJob]:
        """Load all jobs from storage"""
        async with aiosqlite.connect(self.db_path) as db:
            async with db.execute("SELECT * FROM jobs") as cursor:
                rows = await cursor.fetchall()
                jobs = []
                for row in rows:
                    jobs.append(ScheduledJob(
                        id=row[0],
                        name=row[1],
                        cron_expr=row[2],
                        task_description=row[3],
                        status=JobStatus(row[4]),
                        created_at=datetime.fromisoformat(row[5]),
                        last_run=datetime.fromisoformat(row[6]) if row[6] else None,
                        next_run=datetime.fromisoformat(row[7]) if row[7] else None,
                    ))
                return jobs

    async def load_job(self, job_id: str) -> Optional[ScheduledJob]:
        """Load a single job by ID"""
        async with aiosqlite.connect(self.db_path) as db:
            async with db.execute("SELECT * FROM jobs WHERE id = ?", (job_id,)) as cursor:
                row = await cursor.fetchone()
                if row:
                    return ScheduledJob(
                        id=row[0],
                        name=row[1],
                        cron_expr=row[2],
                        task_description=row[3],
                        status=JobStatus(row[4]),
                        created_at=datetime.fromisoformat(row[5]),
                        last_run=datetime.fromisoformat(row[6]) if row[6] else None,
                        next_run=datetime.fromisoformat(row[7]) if row[7] else None,
                    )
                return None

    async def delete_job(self, job_id: str) -> None:
        """Delete a job"""
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute("DELETE FROM jobs WHERE id = ?", (job_id,))
            await db.commit()