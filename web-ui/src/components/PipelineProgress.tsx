import React from 'react';
import { usePipelineStore } from '../stores/pipelineStore';

const stepLabels: Record<string, string> = {
  requirements: '需求澄清',
  patternMatch: '模式匹配',
  knowledgeInject: '知识注入',
  designConfirm: '设计确认',
  build: '生成工作流',
  validate: '验证',
  autoFix: '自动修复',
  credentials: '凭据配置',
  deploy: '部署',
  export: '导出',
};

const statusColors: Record<string, string> = {
  pending: 'bg-gray-200',
  running: 'bg-blue-500 animate-pulse',
  completed: 'bg-green-500',
  failed: 'bg-red-500',
  awaiting: 'bg-yellow-500',
};

export const PipelineProgress: React.FC = () => {
  const { steps, status, design, confirm, reject, error } = usePipelineStore();

  return (
    <div className="p-4 bg-white rounded-lg shadow">
      <h2 className="text-lg font-semibold mb-4">工作流创建流水线</h2>
      
      <div className="flex items-center space-x-2 mb-6 overflow-x-auto pb-2">
        {steps.map((step, idx) => (
          <React.Fragment key={step.name}>
            <div className={`flex items-center px-3 py-2 rounded-full text-sm whitespace-nowrap ${statusColors[step.status]}`}>
              <span className="mr-2">{idx + 1}</span>
              <span>{stepLabels[step.name] || step.name}</span>
              {step.status === 'running' && (
                <span className="ml-2 animate-spin">⏳</span>
              )}
            </div>
            {idx < steps.length - 1 && (
              <div className={`w-8 h-0.5 ${step.status === 'completed' ? 'bg-green-500' : 'bg-gray-300'}`} />
            )}
          </React.Fragment>
        ))}
      </div>

      {status === 'running' && (
        <div className="text-center py-4">
          <div className="animate-spin text-4xl mb-2">⏳</div>
          <p className="text-gray-600">正在处理...</p>
        </div>
      )}

      {status === 'awaiting' && design && (
        <div className="border-2 border-yellow-500 rounded-lg p-4 bg-yellow-50">
          <h3 className="text-lg font-semibold mb-2">⚠️ 请确认工作流设计</h3>
          <div className="bg-white p-3 rounded mb-4 max-h-60 overflow-auto">
            <pre className="text-xs whitespace-pre-wrap">{JSON.stringify(design, null, 2)}</pre>
          </div>
          <div className="flex space-x-2">
            <button 
              onClick={confirm}
              className="bg-green-500 text-white px-4 py-2 rounded hover:bg-green-600"
            >
              ✅ 确认并继续
            </button>
            <button 
              onClick={() => reject('用户拒绝')}
              className="bg-red-500 text-white px-4 py-2 rounded hover:bg-red-600"
            >
              ❌ 重新设计
            </button>
          </div>
        </div>
      )}

      {status === 'completed' && (
        <div className="text-center py-4 bg-green-50 rounded">
          <div className="text-4xl mb-2">🎉</div>
          <p className="text-green-600 font-semibold">工作流创建完成！</p>
        </div>
      )}

      {status === 'failed' && (
        <div className="text-center py-4 bg-red-50 rounded">
          <div className="text-4xl mb-2">❌</div>
          <p className="text-red-600 font-semibold">创建失败</p>
          {error && <p className="text-sm text-red-500 mt-2">{error}</p>}
        </div>
      )}
    </div>
  );
};

export default PipelineProgress;
