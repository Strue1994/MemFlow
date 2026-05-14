import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TaskConsole from '../TaskConsole'
import Layout from '../Layout'
import { taskApi } from '../../api/client'
import { LanguageProvider } from '../../lib/language'

vi.mock('../../api/client', async () => {
  const actual = await vi.importActual<typeof import('../../api/client')>('../../api/client')

  return {
    ...actual,
    taskApi: {
      execute: vi.fn(),
      history: vi.fn(),
    },
  }
})

describe('TaskConsole', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
  })

  afterEach(() => {
    cleanup()
  })

  it('renders the task textarea and routing decision section', () => {
    render(<TaskConsole />)

    expect(screen.getByLabelText(/task request|任务请求/i)).toBeInTheDocument()
    expect(screen.getAllByText(/routing decision|路由决策/i).length).toBeGreaterThan(0)
  })

  it('uses taskApi.execute when the run button is clicked', async () => {
    vi.mocked(taskApi.execute).mockResolvedValue({
      route: 'workflow',
      repeatable: true,
      confidence: 'high',
      reason: 'Matched an existing workflow asset.',
      success: true,
      workflow: { workflowId: 'wf_existing', generated: false },
      result: { status: 'ok' },
    })

    render(<TaskConsole />)

    fireEvent.change(screen.getByLabelText(/task request|任务请求/i), {
      target: { value: 'Generate the daily sales report' },
    })
    fireEvent.click(screen.getAllByRole('button', { name: /run task|运行任务/i })[0])

    await waitFor(() => {
      expect(taskApi.execute).toHaveBeenCalledWith('Generate the daily sales report')
    })
    expect(await screen.findByText(/^Existing workflow$/i)).toBeInTheDocument()
    expect(screen.getByText(/"workflowId": "wf_existing"/i)).toBeInTheDocument()
  })

  it('clears stale result content when a later submission fails', async () => {
    vi.mocked(taskApi.execute)
      .mockResolvedValueOnce({
        route: 'workflow',
        repeatable: true,
        confidence: 'high',
        reason: 'Matched an existing workflow asset.',
        success: true,
        workflow: { workflowId: 'wf_existing', generated: false },
        result: { status: 'ok' },
      })
      .mockRejectedValueOnce(new Error('Backend exploded'))

    render(<TaskConsole />)

    const textarea = screen.getByLabelText(/task request|任务请求/i)
    const button = screen.getAllByRole('button', { name: /run task|运行任务/i })[0]

    fireEvent.change(textarea, { target: { value: 'Generate the daily sales report' } })
    fireEvent.click(button)

    expect(await screen.findByText(/^Existing workflow$/i)).toBeInTheDocument()
    expect(screen.getByText(/"workflowId": "wf_existing"/i)).toBeInTheDocument()

    fireEvent.change(textarea, { target: { value: 'Run the broken task' } })
    fireEvent.click(button)

    expect(await screen.findByText(/backend exploded/i)).toBeInTheDocument()
    await waitFor(() => {
      expect(screen.queryByText(/matched an existing workflow asset/i)).not.toBeInTheDocument()
      expect(screen.queryByText(/"workflowId": "wf_existing"/i)).not.toBeInTheDocument()
      expect(screen.getByText(/awaiting route selection/i)).toBeInTheDocument()
    })
  })

  it('shows the reshaped first-level navigation labels', () => {
    window.localStorage.setItem('memflow-language', 'en')

    render(
      <LanguageProvider>
        <MemoryRouter initialEntries={['/tasks']}>
          <Layout>
            <div>content</div>
          </Layout>
        </MemoryRouter>
      </LanguageProvider>,
    )

    expect(screen.getAllByText('Task Console').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Workflow Assets').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Execution History').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Settings').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Advanced').length).toBeGreaterThan(0)
    expect(screen.getAllByRole('link').some((link) => link.getAttribute('href') === '/tasks')).toBe(true)
    expect(screen.getAllByRole('link').some((link) => link.getAttribute('href') === '/advanced')).toBe(true)
  })

  it('treats advanced child routes as active under the Advanced nav item', () => {
    window.localStorage.setItem('memflow-language', 'en')

    render(
      <LanguageProvider>
        <MemoryRouter initialEntries={['/editor']}>
          <Layout>
            <div>content</div>
          </Layout>
        </MemoryRouter>
      </LanguageProvider>,
    )

    const advancedLinks = screen.getAllByRole('link', { name: /advanced/i })
    expect(advancedLinks.some((link) => link.className.includes('bg-cyan-400/10') || link.className.includes('bg-cyan-400/12'))).toBe(true)
    expect(
      advancedLinks.some((link) => within(link).queryByText('Advanced') !== null),
    ).toBe(true)
  })
})
