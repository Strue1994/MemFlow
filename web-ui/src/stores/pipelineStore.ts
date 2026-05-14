import { create } from 'zustand';

export interface PipelineStep {
  name: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'awaiting';
  message?: string;
  data?: any;
}

export interface PipelineState {
  currentStep: number;
  steps: PipelineStep[];
  status: 'idle' | 'running' | 'awaiting' | 'completed' | 'failed';
  design?: any;
  workflowJson?: any;
  workflowId?: string;
  validationIssues?: ValidationIssue[];
  error?: string;
  
  setCurrentStep: (step: number) => void;
  updateStep: (stepName: string, status: PipelineStep['status'], data?: any) => void;
  setAwaitingConfirmation: (design: any) => void;
  confirm: () => void;
  reject: (reason?: string) => void;
  reset: () => void;
}

export interface ValidationIssue {
  ruleId: string;
  severity: 'error' | 'warning';
  message: string;
  suggestion?: string;
}

const defaultSteps: PipelineStep[] = [
  { name: 'requirements', status: 'pending' },
  { name: 'patternMatch', status: 'pending' },
  { name: 'knowledgeInject', status: 'pending' },
  { name: 'designConfirm', status: 'pending', message: '请确认工作流设计' },
  { name: 'build', status: 'pending' },
  { name: 'validate', status: 'pending' },
  { name: 'autoFix', status: 'pending' },
  { name: 'credentials', status: 'pending' },
  { name: 'deploy', status: 'pending' },
  { name: 'export', status: 'pending' },
];

export const usePipelineStore = create<PipelineState>((set) => ({
  currentStep: 0,
  steps: [...defaultSteps],
  status: 'idle',
  design: undefined,
  workflowJson: undefined,
  workflowId: undefined,
  validationIssues: [],
  error: undefined,

  setCurrentStep: (step: number) => set({ currentStep: step }),

  updateStep: (stepName: string, status: PipelineStep['status'], data?: any) => 
    set((state) => {
      const steps = state.steps.map((s) => 
        s.name === stepName 
          ? { ...s, status, data: data || s.data }
          : s
      );
      
      const currentIdx = steps.findIndex(s => s.name === stepName);
      
      return {
        steps,
        currentStep: currentIdx >= 0 ? currentIdx : state.currentStep,
        status: status === 'running' ? 'running' : state.status,
        ...(data?.design && { design: data.design }),
        ...(data?.workflowJson && { workflowJson: data.workflowJson }),
        ...(data?.workflowId && { workflowId: data.workflowId }),
        ...(data?.validationIssues && { validationIssues: data.validationIssues }),
      };
    }),

  setAwaitingConfirmation: (design: any) => 
    set((state) => ({
      status: 'awaiting',
      design,
      steps: state.steps.map((s) => 
        s.name === 'designConfirm' 
          ? { ...s, status: 'awaiting', data: design }
          : s
      ),
    })),

  confirm: () => 
    set((state) => ({
      status: 'running',
      steps: state.steps.map((s) => 
        s.name === 'designConfirm' 
          ? { ...s, status: 'completed' }
          : s.name === state.steps[state.currentStep]?.name
            ? { ...s, status: 'running' }
            : s
      ),
    })),

  reject: (reason?: string) => 
    set({
      status: 'failed',
      error: reason || '用户拒绝确认',
    }),

  reset: () => 
    set({
      currentStep: 0,
      steps: [...defaultSteps],
      status: 'idle',
      design: undefined,
      workflowJson: undefined,
      workflowId: undefined,
      validationIssues: [],
      error: undefined,
    }),
}));