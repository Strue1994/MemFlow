# P3-3: Marketplace Community Features

## Priority

P3 - Long-term

## Key Files / Modules

- `agent-service/src/marketplace.ts`
- `web-ui/src/components/Marketplace.tsx`
- `executor/src/rating.rs`

## Goals

在现有工作流市场基础上，增加搜索排序、版本兼容性检查、用户评分聚合等功能。

## Specific Requirements

### 1. Search Sorting

- 支持按 "评分" 排序
- 支持按 "下载量" 排序
- 支持按 "最新" 排序
- 支持关键词搜索

```typescript
interface SearchOptions {
  query?: string;
  sortBy: 'rating' | 'downloads' | 'newest';
  sortOrder: 'asc' | 'desc';
}

async function searchWorkflows(options: SearchOptions): Promise<Workflow[]> {
  let workflows = getAllWorkflows();
  
  // Keyword search
  if (options.query) {
    workflows = workflows.filter(w => 
      w.name.toLowerCase().includes(options.query!.toLowerCase()) ||
      w.description.toLowerCase().includes(options.query!.toLowerCase())
    );
  }
  
  // Sort
  switch (options.sortBy) {
    case 'rating':
      workflows.sort((a, b) => b.rating - a.rating);
      break;
    case 'downloads':
      workflows.sort((a, b) => b.downloads - a.downloads);
      break;
    case 'newest':
      workflows.sort((a, b) => b.updatedAt - a.updatedAt);
      break;
  }
  
  if (options.sortOrder === 'asc') {
    workflows.reverse();
  }
  
  return workflows;
}
```

### 2. Version Compatibility Check

- 导入工作流时检查 `memflow_version` 要求
- 检查节点版本兼容性

```typescript
const CURRENT_VERSION = '1.0.0';

interface WorkflowManifest {
  id: string;
  name: string;
  memflow_version?: string;
  nodes?: { type: string; version?: string }[];
}

function checkCompatibility(manifest: WorkflowManifest): CompatibilityResult {
  const errors: string[] = [];
  const warnings: string[] = [];
  
  // Check memflow version
  if (manifest.memflow_version) {
    if (!semver.satisfies(CURRENT_VERSION, manifest.memflow_version)) {
      errors.push(
        `Workflow requires ${manifest.memflow_version}, current is ${CURRENT_VERSION}`
      );
    }
  }
  
  // Check node versions
  if (manifest.nodes) {
    for (const node of manifest.nodes) {
      if (node.version && !isNodeSupported(node.type, node.version)) {
        warnings.push(`Node ${node.type}@${node.version} may not be fully compatible`);
      }
    }
  }
  
  return { compatible: errors.length === 0, errors, warnings };
}
```

### 3. Rating Aggregation

- 列表显示平均分和评分总数
- 评分分布图

```typescript
interface WorkflowRating {
  workflow_id: string;
  average_rating: number;
  total_ratings: number;
  rating_distribution: { 1: number; 2: number; 3: number; 4: number; 5: number };
}

function aggregateRatings(ratings: Rating[]): WorkflowRating {
  const distribution = { 1: 0, 2: 0, 3: 0, 4: 0, 5: 0 };
  
  for (const r of ratings) {
    distribution[r.rating as 1|2|3|4|5]++;
  }
  
  const total = ratings.length;
  const avg = ratings.reduce((sum, r) => sum + r.rating, 0) / total;
  
  return {
    workflow_id: ratings[0].workflow_id,
    average_rating: avg,
    total_ratings: total,
    rating_distribution: distribution,
  };
}
```

### 4. Workflow Version History

- 展示工作流的更新日志和版本号

```typescript
interface WorkflowVersion {
  version: number;
  changelog: string;
  created_at: number;
  author: string;
}

function getVersionHistory(workflowId: string): WorkflowVersion[] {
  return db.query(`
    SELECT version, changelog, created_at, author 
    FROM workflow_versions 
    WHERE workflow_id = ?
    ORDER BY version DESC
  `);
}
```

## Acceptance Criteria

- [ ] 用户可按评分/下载量/最新排序
- [ ] 导入不兼容工作流时有明确警告
- [ ] 评分信息正确显示（平均分、总数、分布）
- [ ] 版本历史可查看

## UI Components

```tsx
// Marketplace.tsx
<SearchBar 
  onSearch={handleSearch}
  sortBy={sortBy}
  onSortChange={setSortBy}
/>

<FilterBar 
  category={category}
  onCategoryChange={setCategory}
/>

{workflows.map(w => (
  <WorkflowCard 
    key={w.id}
    workflow={w}
    rating={aggregateRatings(w.ratings)}
    compatibility={checkCompatibility(w)}
  />
))}

<RatingDistributionChart data={w.rating_distribution} />
<VersionHistoryDialog versions={getVersionHistory(w.id)} />
```