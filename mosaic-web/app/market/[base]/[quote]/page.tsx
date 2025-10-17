import { OrderBook } from "@/components/order-book"
import { Card } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { TrendingUp, TrendingDown } from "lucide-react"

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

const marketData: Record<string, { price: number; change: string; positive: boolean; volume: string }> = {
  "BTC-USDC": { price: 94234.5, change: "+2.34%", positive: true, volume: "$45.2M" },
  "XRP-USD": { price: 2.18, change: "+5.67%", positive: true, volume: "$12.8M" },
  "ETH-USDC": { price: 3456.78, change: "-1.23%", positive: false, volume: "$38.9M" },
  "SOL-USD": { price: 145.32, change: "+8.91%", positive: true, volume: "$23.4M" },
}

export default async function MarketPage({
  params,
}: {
  params: Promise<{ base: string; quote: string }>
}) {
  const { base, quote } = await params
  const marketKey = `${base}-${quote}`
  const market = marketData[marketKey] || { price: 1000, change: "+0.00%", positive: true, volume: "$0" }
  const orderBook = generateOrderBook(market.price)

  return (
    <div className="min-h-screen p-8">
      {/* Market Header */}
      <div className="mb-8">
        <div className="flex items-center gap-3 mb-4">
          <h1 className="text-4xl font-serif text-primary" style={{ fontFamily: "var(--font-playfair)" }}>
            {base}/{quote}
          </h1>
          <Badge variant="outline" className="text-sm">
            OTC Market
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
      </div>

      {/* Order Book */}
      <OrderBook bids={orderBook.bids} asks={orderBook.asks} baseAsset={base} quoteAsset={quote} />
    </div>
  )
}
