/**
 * Skill Loader — SKILL.md compatible loader (Superpowers/agentskills.io format)
 *
 * Scans `skills/` directories for SKILL.md files with YAML frontmatter.
 * Bridges external skill ecosystem into MemFlow's SkillManager.
 *
 * Compatible with: Superpowers, Everything Claude Code, agentskills.io
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { SkillManager, type Skill } from "./skill-system";

interface SkillMeta {
  name: string;
  description: string;
  [key: string]: unknown;
}

/**
 * Parse YAML frontmatter from a markdown file.
 * Simple parser — no external YAML dep needed for the minimal format.
 */
function parseFrontmatter(content: string): { meta: SkillMeta | null; body: string } {
  const match = content.match(/^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$/);
  if (!match) return { meta: null, body: content };

  const yamlBlock = match[1];
  const body = match[2].trim();

  // Minimal YAML parser (handles name + description only)
  const meta: Record<string, unknown> = {};
  for (const line of yamlBlock.split("\n")) {
    const kv = line.match(/^(\w+):\s*(?:"(.+)"|(.+))$/);
    if (kv) {
      meta[kv[1]] = (kv[2] || kv[3] || "").trim();
    }
  }

  return {
    meta: (meta.name ? meta as SkillMeta : null),
    body,
  };
}

/**
 * Recursively scan a directory for SKILL.md files.
 */
export function scanSkillFiles(dir: string): string[] {
  if (!fs.existsSync(dir)) return [];

  const results: string[] = [];

  function walk(current: string) {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (entry.name === "SKILL.md") {
        results.push(fullPath);
      }
    }
  }

  walk(dir);
  return results;
}

/**
 * Load a single SKILL.md file and return a MemFlow Skill object.
 */
export function loadSkillFile(filePath: string): Skill | null {
  try {
    const content = fs.readFileSync(filePath, "utf-8");
    const { meta, body } = parseFrontmatter(content);

    if (!meta) {
      // No frontmatter → use filename as name
      const dirName = path.basename(path.dirname(filePath));
      return {
        id: `ext_${dirName}`,
        name: dirName,
        description: body.split("\n")[0]?.replace(/^#\s*/, "").trim() || dirName,
        category: "imported",
        keywords: [dirName],
        pattern: "",
        steps: [],
        examples: [],
        version: "1.0.0",
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        execution_count: 0,
        success_rate: 0,
      };
    }

    const dirName = path.basename(path.dirname(filePath));
    return {
      id: `ext_${meta.name || dirName}`,
      name: meta.name || dirName,
      description: meta.description || body.split("\n")[0]?.replace(/^#\s*/, "").trim() || dirName,
      category: "superpowers",
      keywords: [meta.name, dirName, ...(meta.name?.split("-") || [])],
      pattern: "",
      steps: [],
      examples: [],
      version: "1.0.0",
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      execution_count: 0,
      success_rate: 0,
    };
  } catch {
    return null;
  }
}

/**
 * Import all SKILL.md files from a directory into SkillManager.
 * Returns the count of imported skills.
 */
export function importSkillsFromDir(
  skillDir: string,
  manager?: SkillManager,
): { imported: number; skills: Skill[] } {
  const mgr = manager || new SkillManager();
  const files = scanSkillFiles(skillDir);
  const skills: Skill[] = [];

  for (const file of files) {
    const skill = loadSkillFile(file);
    if (skill) {
      mgr.saveSkill(skill);
      skills.push(skill);
    }
  }

  return { imported: skills.length, skills };
}

/**
 * Get all configured skill search paths.
 */
export function getSkillSearchPaths(): string[] {
  const paths: string[] = [];

  // Project-level skills
  const cwd = process.cwd();
  const memflowRoot = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(cwd, "..", ".memflow-runtime");

  paths.push(path.resolve(memflowRoot, "skills"));
  paths.push(path.resolve(cwd, "skills"));

  // Claude Code compatibility paths
  if (process.env.HOME) {
    paths.push(path.resolve(process.env.HOME, ".claude", "skills"));
  }

  return paths.filter((p) => fs.existsSync(p));
}

/**
 * Discover all skills from all search paths.
 */
export function discoverAllSkills(manager?: SkillManager): Skill[] {
  const mgr = manager || new SkillManager();
  const allSkills: Skill[] = [];

  for (const searchPath of getSkillSearchPaths()) {
    const result = importSkillsFromDir(searchPath, mgr);
    allSkills.push(...result.skills);
  }

  return allSkills;
}
