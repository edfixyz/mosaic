'use client'

import { OrderBook } from "@/components/order-book"
import { Card } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { TrendingUp, TrendingDown, Loader2 } from "lucide-react"
import { use, useEffect, useState } from "react"
import { getOrImportAccount, getDeskInfo } from "@/lib/account"
import { marketStorage } from "@/lib/marketStorage"

// Generate random order book data
function generateOrderBook(basePrice: number) {
  const bids = Array.from({ length: 15 }, (_, i) => ({
    price: (basePrice * (1 - (i + 1) * 0.001)).toFixed(2),
    amount: (Math.random() * 5 + 0.1).toFixed(4),
    total: (basePrice * (1 - (i + 1) * 0.001) * (Math.random() * 5 + 0.1)).toFixed(2),
  }))

  const asks = Array.from({ length: 15 }, (_, i) => ({
    price: (basePrice * (1 + (i + 1) * 0.001)).toFixed(2),
    amount: (Math.random() * 5 + 0.1).toFixed(4),
    total: (basePrice * (1 + (i + 1) * 0.001) * (Math.random() * 5 + 0.1)).toFixed(2),
  }))

  return { bids, asks }
}

const defaultMarket = { price: 1000, change: "+0.00%", positive: true, volume: "$0" }

export default function MarketPage({
  params,
}: {
  params: Promise<{ marketId: string }>
}) {
  const { marketId } = use(params)
  const [base, setBase] = useState<string>('')
  const [quote, setQuote] = useState<string>('')
  const [baseFaucet, setBaseFaucet] = useState<string>('')
  const [quoteFaucet, setQuoteFaucet] = useState<string>('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const market = defaultMarket
  const orderBook = generateOrderBook(market.price)

  useEffect(() => {
    const loadMarketInfo = async () => {
      try {
        // Dynamically import the SDK
        const { WebClient, AccountId } = await import('@demox-labs/miden-sdk')

        // Initialize client
        const client = await WebClient.createClient()

        // Sync state (may fail for new clients)
        try {
          await client.syncState()
        } catch (syncError) {
          console.warn('Sync state failed:', syncError)
        }

        // Get or import the account
        const account = await getOrImportAccount(client, AccountId, marketId)

        // Get desk info (base and quote symbols)
        const deskInfo = getDeskInfo(account)

        if (deskInfo) {
          setBase(deskInfo.pair.base.symbol)
          setQuote(deskInfo.pair.quote.symbol)
          setBaseFaucet(deskInfo.pair.base.faucet)
          setQuoteFaucet(deskInfo.pair.quote.faucet)

          // Save market to localStorage
          marketStorage.saveMarket({
            pair: `${deskInfo.pair.base.symbol}/${deskInfo.pair.quote.symbol}`,
            marketId,
            baseFaucet: deskInfo.pair.base.faucet,
            quoteFaucet: deskInfo.pair.quote.faucet,
          })
        } else {
          setError('Market not found or invalid market data')
        }
      } catch (error) {
        console.error('Failed to load market info:', error)
        const errorMessage = error instanceof Error ? error.message : 'Failed to load market'
        setError(errorMessage)
      } finally {
        setLoading(false)
      }
    }

    loadMarketInfo()
  }, [marketId])

  return (
    <div className="min-h-screen p-8">
      {/* Market Header */}
      <div className="mb-8">
        {loading ? (
          <Card className="p-8 bg-card border-border">
            <div className="flex flex-col items-center justify-center gap-4">
              <Loader2 className="h-8 w-8 animate-spin text-primary" />
              <p className="text-lg text-muted-foreground">Loading market data...</p>
            </div>
          </Card>
        ) : error ? (
          <Card className="p-8 bg-card border-destructive">
            <div className="text-center">
              <h1 className="text-2xl font-serif text-destructive mb-2" style={{ fontFamily: "var(--font-playfair)" }}>
                Market Not Found
              </h1>
              <p className="text-muted-foreground mb-4">{error}</p>
              <p className="text-sm text-muted-foreground font-mono">
                Market ID: {marketId}
              </p>
            </div>
          </Card>
        ) : (
          <>
            <div className="flex items-center gap-3 mb-4">
              <h1 className="text-4xl font-serif text-primary" style={{ fontFamily: "var(--font-playfair)" }}>
                <span className="relative inline-block group">
                  <span className="cursor-help">{base}</span>
                  {baseFaucet && (
                    <span className="absolute left-1/2 -translate-x-1/2 top-full mt-2 px-3 py-2 bg-popover text-popover-foreground text-xs font-mono rounded-md border border-border shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all whitespace-nowrap z-10">
                      {baseFaucet}
                    </span>
                  )}
                </span>
                /
                <span className="relative inline-block group">
                  <span className="cursor-help">{quote}</span>
                  {quoteFaucet && (
                    <span className="absolute left-1/2 -translate-x-1/2 top-full mt-2 px-3 py-2 bg-popover text-popover-foreground text-xs font-mono rounded-md border border-border shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all whitespace-nowrap z-10">
                      {quoteFaucet}
                    </span>
                  )}
                </span>
              </h1>
              <Badge variant="outline" className="text-sm">
                OTC Market
              </Badge>
              <Badge variant="outline" className="text-sm text-red-500">
                Unverified
              </Badge>
            </div>

            <Card className="p-6 bg-card border-border">
              <div className="grid md:grid-cols-4 gap-6">
                <div>
                  <p className="text-sm text-muted-foreground mb-1">Last Price</p>
                  <p className="text-3xl font-semibold text-foreground">${market.price.toLocaleString()}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground mb-1">24h Change</p>
                  <div
                    className={`flex items-center gap-2 text-2xl font-semibold ${market.positive ? "text-green-500" : "text-red-500"}`}
                  >
                    {market.positive ? <TrendingUp className="h-5 w-5" /> : <TrendingDown className="h-5 w-5" />}
                    {market.change}
                  </div>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground mb-1">24h Volume</p>
                  <p className="text-2xl font-semibold text-foreground">{market.volume}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground mb-1">Market Type</p>
                  <p className="text-2xl font-semibold text-foreground">OTC</p>
                </div>
              </div>
            </Card>
          </>
        )}
      </div>

      {/* Order Book */}
      {!loading && !error && <OrderBook bids={orderBook.bids} asks={orderBook.asks} baseAsset={base} quoteAsset={quote} />}
    </div>
  )
}
