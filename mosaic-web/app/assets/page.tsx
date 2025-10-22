import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Coins } from 'lucide-react'
import clsx from 'clsx'

export type AssetSummary = {
  account: string
  symbol: string
  maxSupply: string
  decimals: number
  verified: boolean
}

async function getAssets(): Promise<AssetSummary[]> {
  const mcpUrl = process.env.NEXT_PUBLIC_MCP_SERVER_URL ?? 'http://localhost:8000/mcp'
  const base = new URL(mcpUrl)
  const assetsUrl = new URL('/assets', `${base.protocol}//${base.host}`)

  const response = await fetch(assetsUrl.toString(), {
    cache: 'no-store',
  })

  if (!response.ok) {
    console.error('Failed to fetch assets', response.status, await response.text())
    throw new Error('Unable to load asset list')
  }

  const data = (await response.json()) as AssetSummary[]
  return data
}

export default async function AssetsPage() {
  const assets = await getAssets()

  return (
    <div className="min-h-screen p-8">
      <div className="mb-8">
        <h1
          className="text-4xl font-serif mb-2 text-primary"
          style={{ fontFamily: 'var(--font-playfair)' }}
        >
          Assets
        </h1>
        <p className="text-muted-foreground">
          Digital assets available for OTC trading on Mosaic
        </p>
      </div>

      <div className="grid gap-4" style={{ fontFamily: 'var(--font-dm-mono)' }}>
        {assets.map((asset) => (
          <Card
            key={asset.account}
            className={clsx(
              'p-6 bg-card border-border transition-colors',
              asset.verified ? 'hover:border-primary/50' : 'border-red-500/50 hover:border-red-500'
            )}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <div className="h-12 w-12 rounded-full bg-primary/10 flex items-center justify-center">
                  <Coins className="h-6 w-6 text-primary" />
                </div>
                <div>
                  <div className="flex items-center gap-3 mb-1">
                    <h3 className="text-xl font-semibold text-foreground">{asset.symbol}</h3>
                    <Badge
                      variant={asset.verified ? 'outline' : 'destructive'}
                      className="text-xs"
                    >
                      {asset.verified ? 'Verified' : 'Unverified'}
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    Account:{' '}
                    {asset.account.startsWith('mtst') ? (
                      <a
                        href={`https://testnet.midenscan.com/account/${asset.account}`}
                        className="text-primary underline-offset-2 hover:underline"
                        rel="noreferrer"
                        target="_blank"
                      >
                        {asset.account}
                      </a>
                    ) : (
                      asset.account
                    )}
                  </p>
                  <p className="text-sm text-muted-foreground">
                    Max Supply: {asset.maxSupply} (decimals: {asset.decimals})
                  </p>
                </div>
              </div>
            </div>
          </Card>
        ))}
      </div>
    </div>
  )
}
