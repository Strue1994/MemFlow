import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'

export type AppLanguage = 'zh' | 'en'

const STORAGE_KEY = 'memflow-language'

const LanguageContext = createContext<{
  language: AppLanguage
  setLanguage: (language: AppLanguage) => void
  text: (copy: { zh: string; en: string }) => string
} | null>(null)

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<AppLanguage>(() => {
    const stored = localStorage.getItem(STORAGE_KEY)
    return stored === 'en' ? 'en' : 'zh'
  })

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, language)
  }, [language])

  const value = useMemo(
    () => ({
      language,
      setLanguage: (nextLanguage: AppLanguage) => setLanguageState(nextLanguage),
      text: (copy: { zh: string; en: string }) => (language === 'zh' ? copy.zh : copy.en),
    }),
    [language],
  )

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>
}

export function useLanguage() {
  const context = useContext(LanguageContext)
  if (!context) {
    throw new Error('useLanguage must be used within LanguageProvider')
  }
  return context
}
