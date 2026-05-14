import { lazy, Suspense } from 'react'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Toaster } from 'sonner'
import Layout from './components/Layout'
import { LanguageProvider } from './lib/language'

const Dashboard = lazy(() => import('./components/Dashboard'))
const TaskConsole = lazy(() => import('./components/TaskConsole'))
const WorkflowAssetsPage = lazy(() => import('./components/WorkflowAssetsPage'))
const TaskHistoryPage = lazy(() => import('./components/TaskHistoryPage'))
const WorkflowEditor = lazy(() => import('./components/WorkflowEditor'))
const Marketplace = lazy(() => import('./components/Marketplace'))
const Settings = lazy(() => import('./components/Settings'))
const EvolutionTimeline = lazy(() => import('./components/EvolutionTimeline'))
const ComputerAgent = lazy(() => import('./components/ComputerAgent'))
const NLCreator = lazy(() => import('./components/NLCreator'))

function Loading() {
  return (
    <div className="flex items-center justify-center h-full py-20">
      <div className="text-gray-500 animate-pulse">加载中...</div>
    </div>
  )
}

function App() {
  return (
    <LanguageProvider>
      <BrowserRouter>
        <Toaster richColors position="bottom-right" />
        <Layout>
          <Suspense fallback={<Loading />}>
            <Routes>
              <Route path="/" element={<Navigate to="/tasks" replace />} />
              <Route path="/tasks" element={<TaskConsole />} />
              <Route path="/assets" element={<WorkflowAssetsPage />} />
              <Route path="/history" element={<TaskHistoryPage />} />
              <Route path="/advanced" element={<Dashboard />} />
              <Route path="/dashboard" element={<Navigate to="/advanced" replace />} />
              <Route path="/editor" element={<WorkflowEditor />} />
              <Route path="/create" element={<NLCreator />} />
              <Route path="/computer" element={<ComputerAgent />} />
              <Route path="/logs" element={<Navigate to="/history" replace />} />
              <Route path="/marketplace" element={<Marketplace />} />
              <Route path="/settings" element={<Settings />} />
              <Route path="/timeline" element={<EvolutionTimeline />} />
              <Route path="*" element={<Navigate to="/tasks" replace />} />
            </Routes>
          </Suspense>
        </Layout>
      </BrowserRouter>
    </LanguageProvider>
  )
}

export default App
