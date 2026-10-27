import { useEffect, useState } from 'react'
import * as api from './api'

export default function App() {
  const [version, setVersion] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    api
      .appInfo()
      .then((info) => {
        if (!cancelled) setVersion(info.version)
      })
      .catch(() => {
        if (!cancelled) setVersion('')
      })
    return () => {
      cancelled = true
    }
  }, [])

  return (
    <main>
      <h1>{version ? `Keepers ${version}` : 'Keepers'}</h1>
    </main>
  )
}
