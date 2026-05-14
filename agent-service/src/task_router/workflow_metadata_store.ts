import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import type { WorkflowAssetMetadata } from "./types";

function compareMetadataRecords(
  left: WorkflowAssetMetadata,
  right: WorkflowAssetMetadata,
): number {
  return left.workflowId.localeCompare(right.workflowId);
}

export class FileWorkflowMetadataStore {
  private writeQueue: Promise<void> = Promise.resolve();

  constructor(private readonly filePath: string) {}

  async list(): Promise<WorkflowAssetMetadata[]> {
    try {
      const content = await readFile(this.filePath, "utf8");
      const parsed = JSON.parse(content);
      return Array.isArray(parsed) ? (parsed as WorkflowAssetMetadata[]) : [];
    } catch (error: unknown) {
      if ((error as NodeJS.ErrnoException)?.code === "ENOENT") {
        return [];
      }

      throw error;
    }
  }

  async upsert(next: WorkflowAssetMetadata): Promise<void> {
    const operation = this.writeQueue.then(async () => {
      const existing = await this.list();
      const filtered = existing.filter(
        (record) => record.workflowId !== next.workflowId,
      );
      const ordered = [...filtered, next].sort(compareMetadataRecords);

      await mkdir(path.dirname(this.filePath), { recursive: true });
      await writeFile(this.filePath, `${JSON.stringify(ordered, null, 2)}\n`, "utf8");
    });

    this.writeQueue = operation.then(
      () => undefined,
      () => undefined,
    );

    return operation;
  }
}
