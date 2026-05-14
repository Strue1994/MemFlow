import { lazy, Suspense, useState } from 'react'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Toaster } from 'sonner'
import Layout from './components/Layout'
import { ErrorBoundary } from './components/ErrorBoundary'
import { LoginPage } from './components/LoginPage'
import { LoadingSkeleton } from './components/LoadingSkeleton'
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

function Loading() { return <LoadingSkeleton lines={4} />; }

function App() {
  return (
    <LanguageProvider>
      <BrowserRouter>
        <Toaster richColors position="bottom-right" />
        <ErrorBoundary><Layout>
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
        </Layout></ErrorBoundary>
      </BrowserRouter>
    </LanguageProvider>
  )
}

export default App

