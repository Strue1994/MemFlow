import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import type { HistoricalTaskRecord } from "./types";

function compareHistoryRecords(
  left: HistoricalTaskRecord,
  right: HistoricalTaskRecord,
): number {
  const leftTime = Date.parse(left.createdAt);
  const rightTime = Date.parse(right.createdAt);

  if (Number.isNaN(leftTime) || Number.isNaN(rightTime)) {
    return right.createdAt.localeCompare(left.createdAt);
  }

  return rightTime - leftTime;
}

export class FileTaskHistoryStore {
  private writeQueue: Promise<void> = Promise.resolve();

  constructor(private readonly filePath: string) {}

  async list(): Promise<HistoricalTaskRecord[]> {
    try {
      const content = await readFile(this.filePath, "utf8");
      const parsed = JSON.parse(content);
      return Array.isArray(parsed) ? (parsed as HistoricalTaskRecord[]) : [];
    } catch (error: unknown) {
      if ((error as NodeJS.ErrnoException)?.code === "ENOENT") {
        return [];
      }

      throw error;
    }
  }

  async append(record: HistoricalTaskRecord): Promise<void> {
    const operation = this.writeQueue.then(async () => {
      const existing = await this.list();
      const next = [record, ...existing].sort(compareHistoryRecords).slice(0, 100);

      await mkdir(path.dirname(this.filePath), { recursive: true });
      await writeFile(this.filePath, `${JSON.stringify(next, null, 2)}\n`, "utf8");
    });

    this.writeQueue = operation.then(
      () => undefined,
      () => undefined,
    );

    return operation;
  }
}
