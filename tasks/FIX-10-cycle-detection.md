# FIX-10: Add Cycle Detection for Workflows

## 问题描述

Workflow 中可能存在循环依赖，导致无限循环执行。需要添加循环检测。

## 检查问题

在 workflow 执行器中检查:

```rust
// 可能的循环
node_a -> node_b -> node_c -> node_a
```

## 修复方案

1. 添加拓扑排序检测:
   ```rust
   pub fn detect_cycle(nodes: &[WorkflowNode], connections: &[Connection]) -> bool {
       let mut graph: HashMap<String, Vec<String>> = HashMap::new();
       
       for conn in connections {
           graph.entry(conn.source.clone())
               .or_default()
               .push(conn.target.clone());
       }
       
       let mut visited = HashSet::new();
       let mut recursion_stack = HashSet::new();
       
       fn has_cycle_inner(
           node: &str,
           graph: &HashMap<String, Vec<String>>,
           visited: &mut HashSet<String>,
           recursion_stack: &mut HashSet<String>,
       ) -> bool {
           if recursion_stack.contains(node) {
               return true;
           }
           if visited.contains(node) {
               return false;
           }
           
           visited.insert(node.to_string());
           recursion_stack.insert(node.to_string());
           
           if let Some(neighbors) = graph.get(node) {
               for neighbor in neighbors {
                   if has_cycle_inner(neighbor, graph, visited, recursion_stack) {
                       return true;
                   }
               }
           }
           
           recursion_stack.remove(node);
           false
       }
       
       for node in graph.keys() {
           if has_cycle_inner(node, &graph, &mut visited, &mut recursion_stack) {
               return true;
           }
       }
       
       false
   }
   ```

2. 在执行前检测:
   ```rust
   pub fn execute_workflow(workflow: &Workflow) -> Result<Value> {
       if detect_cycle(&workflow.nodes, &workflow.connections) {
           return Err(ExecError::ValidationError("Cycle detected in workflow".to_string()));
       }
       // 执行...
   }
   ```

## 影响文件

- `executor/src/lib.rs` (或工作流执行模块)

## 验证方法

创建有循环的 workflow，确认被拒绝执行。

## 优先级

HIGH - 稳定性问题