import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Coins, AlertCircle } from 'lucide-react'
import clsx from 'clsx'
import { AssetSummary, callMcpTool } from '@/lib/mcp-tool'
import { formatAssetSupply } from '@/lib/asset-format'
import { getServerAccessToken } from '@/lib/server-auth'

export const dynamic = 'force-dynamic'

async function getAssets(): Promise<AssetSummary[] | null> {
  try {
    const accessToken = await getServerAccessToken()
    if (accessToken) {
      const assets = await callMcpTool('list_assets', {}, accessToken)
      return assets
    }

    const mcpUrl = process.env.NEXT_PUBLIC_MCP_SERVER_URL ?? 'http://localhost:8000/mcp'
    const base = new URL(mcpUrl)
    const assetsUrl = new URL('/assets', `${base.protocol}//${base.host}`)
    const response = await fetch(assetsUrl.toString(), { cache: 'no-store' })

    if (!response.ok) {
      console.error('Failed to fetch public assets', response.status, await response.text())
      return null
    }

    const assets = (await response.json()) as AssetSummary[]
    return assets
  } catch (error) {
    console.error('Failed to load assets via MCP:', error)
    return null
  }
}

export default async function AssetsPage() {
  const assets = await getAssets()

  if (!assets) {
    return (
      <Card className="p-8 bg-card border-border">
        <div className="flex flex-col items-center justify-center gap-4 text-center">
          <div className="h-16 w-16 rounded-full bg-destructive/10 flex items-center justify-center">
            <AlertCircle className="h-8 w-8 text-destructive" />
          </div>
          <div>
            <h3 className="text-xl font-semibold text-foreground mb-2">Unable to Load Assets</h3>
            <p className="text-muted-foreground max-w-md">
              We couldn&apos;t retrieve the asset list at this time. The server may be unavailable or
              the request timed out. Please try refreshing the page.
            </p>
          </div>
        </div>
      </Card>
    )
  }

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
              <Badge variant={asset.verified ? 'outline' : 'destructive'} className="text-xs">
                {asset.verified ? 'Verified' : 'Unverified'}
              </Badge>
              {asset.owner && (
                <Badge variant="outline" className="text-xs">
                  Owner
                </Badge>
              )}
              {asset.account.startsWith('mtst') && (
                <Badge variant="outline" className="text-xs">
                  Testnet
                </Badge>
              )}
              <Badge variant="outline" className="text-xs">
                Public
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
                    Max Supply: {formatAssetSupply(asset.maxSupply, asset.decimals)} (decimals: {asset.decimals})
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
