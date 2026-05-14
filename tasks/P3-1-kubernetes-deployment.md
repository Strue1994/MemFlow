# P3-1: Kubernetes Deployment (Helm Chart)

## Priority

P3 - Long-term

## Key Files / Modules

- `deploy/helm/memflow/` (new directory)
- `deploy/helm/memflow/Chart.yaml`
- `deploy/helm/memflow/values.yaml`

## Goals

支持在 K8s 集群中部署 MemFlow，实现自动伸缩和高可用。

## Specific Requirements

1.  **Helm Chart Structure**
   ```
   deploy/helm/memflow/
   ├── Chart.yaml
   ├── templates/
   │   ├── executor-deployment.yaml
   │   ├── agent-deployment.yaml
   │   ├── redis-deployment.yaml
   │   ├── postgres-deployment.yaml
   │   ├── service.yaml
   │   └── networkpolicy.yaml
   └── values.yaml
   ```

2.  **HPA (Horizontal Pod Autoscaler)**
   - 根据 CPU 自动扩缩 executor
   - 根据并发工作流数自定义指标

3.  **PDB (Pod Disruption Budget)**
   - 保证升级时最小可用实例

4.  **NetworkPolicy**
   - 限制服务间网络访问

5.  **Resource Limits**
   - 为所有组件设置资源限制

## Acceptance Criteria

- [ ] `helm install` 后所有服务正常运行
- [ ] 可通过 `kubectl scale` 手动扩缩

## Implementation

```yaml
# values.yaml
replicaCount: 3

executor:
  resources:
    limits:
      cpu: "2"
      memory: 2Gi
    requests:
      cpu: "500m"
      memory: 512Mi
  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 10
    targetCPUUtilizationPercentage: 70

agent:
  resources:
    limits:
      cpu: "1"
      memory: 1Gi

redis:
  resources:
    limits:
      cpu: "500m"
      memory: 512Mi

podDisruptionBudget:
  minAvailable: 2

networkPolicy:
  enabled: true
```

```bash
# Installation
helm install memflow ./deploy/helm/memflow
```