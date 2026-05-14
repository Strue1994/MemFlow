const API_KEY_EVENT = 'memflow-api-key-changed'

export function getStoredApiKey(): string {
  return localStorage.getItem('apiKey') || ''
}

export function setStoredApiKey(value: string): void {
  const trimmed = value.trim()
  if (trimmed) {
    localStorage.setItem('apiKey', trimmed)
  } else {
    localStorage.removeItem('apiKey')
  }
  window.dispatchEvent(new CustomEvent(API_KEY_EVENT, { detail: trimmed }))
}

export function subscribeToApiKey(callback: (value: string) => void): () => void {
  const handler = () => callback(getStoredApiKey())
  window.addEventListener(API_KEY_EVENT, handler)
  window.addEventListener('storage', handler)
  return () => {
    window.removeEventListener(API_KEY_EVENT, handler)
    window.removeEventListener('storage', handler)
  }
}
