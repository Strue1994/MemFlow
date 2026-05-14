# FIX-06: Add Input Validation to Agent Service

## 问题描述

Agent service (Express.js) 的 API 端点缺少输入验证，可能导致:
- 恶意输入导致 crash
- SQL injection
- XSS 攻击
- 拒绝服务

## 当前端点 (agent-service/src/index.ts)

缺少 validation middleware:

```typescript
app.post('/api/workflow/execute', async (req, res) => {
  const { workflow } = req.body;  // 没有验证
  // 直接使用 workflow
});
```

## 修复方案

1. 添加 Zod 验证 schemas:
   ```typescript
   import { z } from 'zod';
   
   const WorkflowSchema = z.object({
     name: z.string().min(1).max(100),
     nodes: z.array(NodeSchema),
     connections: z.array(ConnectionSchema).optional(),
     settings: WorkflowSettingsSchema.optional(),
   });
   
   const ExecuteRequestSchema = z.object({
     workflow: WorkflowSchema,
     input: z.record(z.unknown()).optional(),
   });
   ```

2. 添加验证中间件:
   ```typescript
   function validateBody<T>(schema: z.ZodSchema<T>) {
     return (req, res, next) => {
       const result = schema.safeParse(req.body);
       if (!result.success) {
         return res.status(400).json({ error: result.error });
       }
       req.validated = result.data;
       next();
     };
   }
   
   app.post('/api/workflow/execute', 
     validateBody(ExecuteRequestSchema),
     async (req, res) => { ... }
   );
   ```

3. 添加大小限制:
   ```typescript
   app.use(express.json({ limit: '1mb' }));
   app.use(express.urlencoded({ limit: '1mb', extended: false }));
   ```

## 影响文件

- `agent-service/src/index.ts`

## 验证方法

1. 发送恶意输入测试端点
2. 检查请求体大小限制生效

## 优先级

HIGH - 安全漏洞