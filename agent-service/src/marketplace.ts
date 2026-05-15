/**
 * Skill Marketplace — agentskills.io compatible skill distribution
 *
 * Features:
 * - Install skills from GitHub repos
 * - List available skills from registry
 * - Publish skills to marketplace
 * - agentskills.io metadata format support
 */

import * as fs from "node:fs";
import * as path from "node:path";

// ---- Types ----

export interface MarketplaceListing {
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  source: string;       // GitHub URL
  category: string;
  tags: string[];
  downloads: number;
  rating: number;
  updatedAt: string;
}

export interface InstalledPlugin {
  name: string;
  source: string;
  version: string;
  installedAt: string;
  skills: string[];
}

// ---- Paths ----

function getMarketplaceDir(): string {
  const root = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
  return path.resolve(root, "marketplace");
}

function getInstalledPath(): string {
  return path.resolve(getMarketplaceDir(), "installed.json");
}

function getSkillsDir(): string {
  const root = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
  return path.resolve(root, "skills");
}

// ---- Registry (curated list of known skill sources) ----

const REGISTRY: MarketplaceListing[] = [
  {
    id: "superpowers-core",
    name: "Superpowers Core",
    description: "Core skills from the Superpowers framework: brainstorming, TDD, debugging, writing-plans, verification",
    author: "obra",
    version: "5.1.0",
    source: "https://github.com/obra/superpowers",
    category: "methodology",
    tags: ["tdd", "debugging", "planning", "brainstorming"],
    downloads: 190000,
    rating: 4.8,
    updatedAt: "2026-05-04",
  },
  {
    id: "ecc-core",
    name: "ECC Core Skills",
    description: "Everything Claude Code core skills: 200+ production-ready agent skills",
    author: "affaan-m",
    version: "1.10.0",
    source: "https://github.com/affaan-m/everything-claude-code",
    category: "general",
    tags: ["code-review", "testing", "security", "optimization"],
    downloads: 180000,
    rating: 4.7,
    updatedAt: "2026-04-05",
  },
  {
    id: "memflow-starter",
    name: "MemFlow Starter Pack",
    description: "Official MemFlow starter skills: workflow automation, memory management, tool usage patterns",
    author: "memflow",
    version: "1.0.0",
    source: "built-in",
    category: "memflow",
    tags: ["workflow", "automation", "memflow"],
    downloads: 0,
    rating: 5.0,
    updatedAt: new Date().toISOString().split("T")[0],
  },
];

// ---- Public API ----

/** List all available marketplace listings */
export function listMarketplace(): MarketplaceListing[] {
  return [...REGISTRY];
}

/** Search marketplace listings */
export function searchMarketplace(query: string): MarketplaceListing[] {
  const q = query.toLowerCase();
  return REGISTRY.filter(
    (item) =>
      item.name.toLowerCase().includes(q) ||
      item.description.toLowerCase().includes(q) ||
      item.tags.some((t) => t.includes(q)),
  );
}

/** Get installed plugins */
export function getInstalled(): InstalledPlugin[] {
  try {
    const raw = fs.readFileSync(getInstalledPath(), "utf-8");
    return JSON.parse(raw);
  } catch {
    return [];
  }
}

/** Save installed plugins list */
function saveInstalled(plugins: InstalledPlugin[]): void {
  const dir = getMarketplaceDir();
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(getInstalledPath(), JSON.stringify(plugins, null, 2), "utf-8");
}

/** Install a skill from the marketplace */
export async function installSkill(id: string): Promise<{ success: boolean; skills: string[]; error?: string }> {
  const listing = REGISTRY.find((l) => l.id === id);
  if (!listing) return { success: false, skills: [], error: `Unknown skill: ${id}` };

  if (listing.source === "built-in") {
    // Built-in skills are already available via SKILL.md discovery
    const installed = getInstalled();
    if (!installed.find((p) => p.name === listing.id)) {
      installed.push({
        name: listing.id,
        source: "built-in",
        version: listing.version,
        installedAt: new Date().toISOString(),
        skills: [listing.name],
      });
      saveInstalled(installed);
    }
    return { success: true, skills: [listing.name] };
  }

  // For remote sources, download and install
  try {
    const skillsDir = getSkillsDir();
    if (!fs.existsSync(skillsDir)) fs.mkdirSync(skillsDir, { recursive: true });

    // Download SKILL.md files from the GitHub repo
    const repoPath = listing.source.replace("https://github.com/", "");
    const apiUrl = `https://api.github.com/repos/${repoPath}/contents/skills`;

    const resp = await fetch(apiUrl, {
      headers: { "Accept": "application/vnd.github.v3+json", "User-Agent": "memflow-marketplace" },
    });

    if (!resp.ok) {
      return { success: false, skills: [], error: `GitHub API error: ${resp.status}` };
    }

    const items = await resp.json() as Array<{ name: string; type: string; url: string }>;
    const skillDirs = items.filter((i) => i.type === "dir");
    const installed: string[] = [];

    for (const skillDir of skillDirs) {
      const skillResp = await fetch(
        `https://raw.githubusercontent.com/${repoPath}/main/skills/${skillDir.name}/SKILL.md`,
        { headers: { "User-Agent": "memflow-marketplace" } },
      );

      if (skillResp.ok) {
        const content = await skillResp.text();
        const targetDir = path.join(skillsDir, skillDir.name);
        if (!fs.existsSync(targetDir)) fs.mkdirSync(targetDir, { recursive: true });
        fs.writeFileSync(path.join(targetDir, "SKILL.md"), content, "utf-8");
        installed.push(skillDir.name);
      }
    }

    // Record installation
    const installedPlugins = getInstalled();
    installedPlugins.push({
      name: listing.id,
      source: listing.source,
      version: listing.version,
      installedAt: new Date().toISOString(),
      skills: installed,
    });
    saveInstalled(installedPlugins);

    return { success: true, skills: installed };
  } catch (err: any) {
    return { success: false, skills: [], error: err.message };
  }
}

/** Uninstall a skill */
export function uninstallSkill(id: string): boolean {
  const installed = getInstalled();
  const idx = installed.findIndex((p) => p.name === id);
  if (idx < 0) return false;

  // Remove skill files
  const skillsDir = getSkillsDir();
  for (const skillName of installed[idx].skills) {
    const skillPath = path.join(skillsDir, skillName);
    try {
      if (fs.existsSync(skillPath)) {
        fs.rmSync(skillPath, { recursive: true, force: true });
      }
    } catch { /* best-effort */ }
  }

  installed.splice(idx, 1);
  saveInstalled(installed);
  return true;
}
