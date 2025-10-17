'use client'

import { useEffect, useState } from 'react'
import { getOrImportAccount, readAccountStorage, getDeskInfo } from '@/lib/account'

const Page = () => {
  const [error, setError] = useState<string | null>(null)
  const [status, setStatus] = useState<string>('Initializing...')

  const run = async () => {
    try {
      setStatus('Loading Miden SDK...')
      //indexedDB.deleteDatabase('MidenClientDB')
      // Dynamically import the Miden SDK only on the client side
      const { AccountId, WebClient, Word, Felt } = await import('@demox-labs/miden-sdk')

      setStatus('Creating client...')
      const client = await WebClient.createClient()

      setStatus('Syncing state...')
      try {
        await client.syncState()
      } catch (syncError) {
        console.warn('Sync state failed, this is expected for new clients:', syncError)
        // Continue even if sync fails - this is expected for fresh databases
      }

      setStatus('Getting account...')
      const account = await getOrImportAccount(
        client,
        AccountId,
        'mtst1qrf9y8pmykxfyqppuehkvds3ffcqqdtepua'
      )

      const deskInfo = getDeskInfo(account)

      if (deskInfo) {
        console.log('DDD', deskInfo)
        setStatus('Account loaded successfully!')
      } else {
        setStatus('Account not found')
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      console.error('Error:', err)
      setError(errorMessage)
      setStatus('Error occurred')
    }
  }

  useEffect(() => {
    run()
  }, [])

  return (
    <div style={{ padding: '20px' }}>
      <h1>Miden SDK Test</h1>
      <p>Status: {status}</p>
      {error && (
        <div style={{ color: 'red', marginTop: '10px' }}>
          <strong>Error:</strong> {error}
          <p style={{ fontSize: '12px', marginTop: '5px' }}>
            Try clearing IndexedDB in DevTools → Application → Storage → IndexedDB
          </p>
        </div>
      )}
    </div>
  )
}

export default Page