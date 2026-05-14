# P1-1: Compiler Workflow Cycle Detection

## Priority

P1 - Near-term

## Key Files / Modules

- `compiler/src/parser.rs`
- `compiler/src/graph.rs` (new)
- `compiler/src/error.rs`

## Goals

在编译阶段检测 `CallWorkflow` 循环调用，防止运行时无限递归。

## Specific Requirements

1.  **Build Call Graph**
   - 解析工作流中的所有 `CallWorkflow` 节点
   - 构建被调用工作流的依赖有向图

2.  **Cycle Detection Algorithm**
   - 使用 DFS (深度优先搜索) 或拓扑排序
   - 检测图中的环

3.  **Clear Error Message**
   - 若检测到环，返回清晰错误
   - 指明循环路径上的所有节点

## Acceptance Criteria

- [ ] 包含循环调用的工作流编译失败
- [ ] 错误信息包含循环路径

## Implementation

```rust
pub fn detect_call_cycle(workflows: &[Workflow]) -> Result<(), CompilerError> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    
    for wf in workflows {
        for node in &wf.nodes {
            if let Some(call) = node.as_call_workflow() {
                graph.entry(wf.id.clone()).or_default().push(call.target.clone());
            }
        }
    }
    
    // DFS cycle detection
    let mut visited = HashSet::new();
    let mut recursion = HashSet::new();
    
    fn dfs(node: &str, graph: &Graph, visited: &mut HashSet, recursion: &mut HashSet) -> bool {
        if recursion.contains(node) { return true; }
        if visited.contains(node) { return false; }
        
        visited.insert(node.to_string());
        recursion.insert(node.to_string());
        
        if let Some(deps) = graph.get(node) {
            for dep in deps {
                if dfs(dep, graph, visited, recursion) { return true; }
            }
        }
        recursion.remove(node);
        false
    }
    
    for node in graph.keys() {
        if dfs(node, &graph, &mut visited, &mut recursion) {
            return Err(CompilerError::Cycle(format!("Cycle detected: {:?}", recursion)));
        }
    }
    Ok(())
}
```