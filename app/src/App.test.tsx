import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import App from './App'

vi.mock('./api', () => ({
  appInfo: vi.fn().mockResolvedValue({ version: '0.1.0' }),
}))

describe('App', () => {
  it('renders the name and the version from app_info', async () => {
    render(<App />)
    expect(await screen.findByText('Keepers 0.1.0')).toBeDefined()
  })
})
