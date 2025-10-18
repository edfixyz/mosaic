'use client'

import { useEffect, useState } from 'react'
import { bech32m } from 'bech32'

const accountIdFromBehc32 = (addr: string): { hex: string, prefix: string } => {
  const { words, prefix } = bech32m.decode(addr)
  const full = Uint8Array.from(bech32m.fromWords(words))
  const noLead = (full.length > 0 && full[0] === 0x00) ? full.slice(1) : full
  const accountId = noLead.slice(0, 15)
  return { hex: Buffer.from(accountId).toString('hex'), prefix }
}

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
            //const accountId0 = AccountId.fromHex('0x2e73ca8238a99e0067cfe29918f049')
            const accountIdRaw = accountIdFromBehc32('mtst1qqh88j5z8z5euqr8el3fjx8sf9cqqeuzsuj')
            const accountIHex = '0x' + accountIdRaw.hex
            console.log(accountIHex)
            const accountId = AccountId.fromHex(accountIHex)
            let account = await client.getAccount(accountId)
            if(account) {
                // TODO
                console.log('ACC', account)
            } else {
                account = await client.importAccountById(accountId)
            }

            if(account) {
                //const z = new Felt(BigInt(0))
                //const v = account.storage().getMapItem(1, Word.newFromFelts([z,z,z,z]))
                //console.log('Value v', v)
                const w0 = account.storage().getItem(0)
                const w2 = account.storage().getItem(2)
                console.log('W0:', w0?.toFelts().map((x) => x.asInt()))
                console.log('W2:', w2?.toFelts().map((x) => x.asInt()))
                console.log('Account ID:', account.id().toString())
                console.log('Nonce:', account.nonce().toString())
                console.log('Commitment:', account.commitment().toHex())
                console.log('Is Public:', account.isPublic())
                console.log('Is Updatable:', account.isUpdatable())
                console.log('Is Faucet:', account.isFaucet())
                console.log('Is Regular Account:', account.isRegularAccount())
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