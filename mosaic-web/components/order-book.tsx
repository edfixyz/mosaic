import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"

interface Order {
  price: string
  amount: string
  total: string
}

interface OrderBookProps {
  bids: Order[]
  asks: Order[]
  baseAsset: string
  quoteAsset: string
  canRequestQuote?: boolean
  canOfferLiquidity?: boolean
  onRequestQuote?: (side: 'Buy' | 'Sell') => void
  onOfferLiquidity?: (side: 'Buy' | 'Sell') => void
}

export function OrderBook({
  bids,
  asks,
  baseAsset,
  quoteAsset,
  canRequestQuote = false,
  canOfferLiquidity = false,
  onRequestQuote,
  onOfferLiquidity,
}: OrderBookProps) {
  const maxBidAmount = bids.reduce((max, bid) => Math.max(max, Number(bid.amount) || 0), 0)
  const maxAskAmount = asks.reduce((max, ask) => Math.max(max, Number(ask.amount) || 0), 0)

  return (
    <div className="grid lg:grid-cols-2 gap-6" style={{ fontFamily: "var(--font-dm-mono)" }}>
      {/* Bids (Buy Orders) */}
      <Card className="p-6 bg-card border-border">
        <div className="mb-4 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <h2 className="text-xl font-semibold text-green-500">Bids (Buy Orders)</h2>
            <div className="flex items-center gap-2">
              {canOfferLiquidity && (
                <Button
                  size="sm"
                  variant="outline"
                  className="border border-border"
                  onClick={() => onOfferLiquidity?.('Sell')}
                >
                  Offer Liquidity
                </Button>
              )}
              {canRequestQuote && (
                <Button
                  size="sm"
                  variant="outline"
                  className="border border-border"
                  onClick={() => onRequestQuote?.('Sell')}
                >
                  Request Quote
                </Button>
              )}
            </div>
          </div>
          <p className="text-sm text-muted-foreground">
            Orders to buy {baseAsset} with {quoteAsset}
          </p>
        </div>

        <div className="space-y-2">
          {/* Header */}
          <div className="grid grid-cols-3 gap-4 pb-2 border-b border-border text-xs text-muted-foreground font-medium">
            <div>Price ({quoteAsset})</div>
            <div className="text-right">Amount ({baseAsset})</div>
            <div className="text-right">Total ({quoteAsset})</div>
          </div>

          {/* Bid Orders */}
          <div className="space-y-1">
            {bids.length === 0 ? (
              <div className="py-8 text-center text-muted-foreground">
                No buy orders available
              </div>
            ) : (
              bids.map((bid, index) => {
                const amountValue = Number(bid.amount) || 0
                const fillPercent = maxBidAmount > 0 ? Math.min((amountValue / maxBidAmount) * 100, 100) : 0

                return (
                  <div
                    key={index}
                    className="grid grid-cols-3 gap-4 py-2 hover:bg-secondary/50 rounded transition-colors relative overflow-hidden"
                  >
                    <div
                      className="absolute inset-y-0 left-0 bg-green-500/10"
                      style={{ width: `${fillPercent}%` }}
                    />
                    <div className="relative text-green-500 font-mono text-sm">{bid.price}</div>
                    <div className="relative text-right text-foreground font-mono text-sm">{bid.amount}</div>
                    <div className="relative text-right text-muted-foreground font-mono text-sm">{bid.total}</div>
                  </div>
                )
              })
            )}
          </div>
        </div>
      </Card>

      {/* Asks (Sell Orders) */}
      <Card className="p-6 bg-card border-border">
        <div className="mb-4 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <h2 className="text-xl font-semibold text-red-500">Asks (Sell Orders)</h2>
            <div className="flex items-center gap-2">
              {canOfferLiquidity && (
                <Button
                  size="sm"
                  variant="outline"
                  className="border border-border"
                  onClick={() => onOfferLiquidity?.('Buy')}
                >
                  Offer Liquidity
                </Button>
              )}
              {canRequestQuote && (
                <Button
                  size="sm"
                  variant="outline"
                  className="border border-border"
                  onClick={() => onRequestQuote?.('Buy')}
                >
                  Request Quote
                </Button>
              )}
            </div>
          </div>
          <p className="text-sm text-muted-foreground">
            Orders to sell {baseAsset} for {quoteAsset}
          </p>
        </div>

        <div className="space-y-2">
          {/* Header */}
          <div className="grid grid-cols-3 gap-4 pb-2 border-b border-border text-xs text-muted-foreground font-medium">
            <div>Price ({quoteAsset})</div>
            <div className="text-right">Amount ({baseAsset})</div>
            <div className="text-right">Total ({quoteAsset})</div>
          </div>

          {/* Ask Orders */}
          <div className="space-y-1">
            {asks.length === 0 ? (
              <div className="py-8 text-center text-muted-foreground">
                No sell orders available
              </div>
            ) : (
              asks.map((ask, index) => {
                const amountValue = Number(ask.amount) || 0
                const fillPercent = maxAskAmount > 0 ? Math.min((amountValue / maxAskAmount) * 100, 100) : 0

                return (
                  <div
                    key={index}
                    className="grid grid-cols-3 gap-4 py-2 hover:bg-secondary/50 rounded transition-colors relative overflow-hidden"
                  >
                    <div
                      className="absolute inset-y-0 left-0 bg-red-500/10"
                      style={{ width: `${fillPercent}%` }}
                    />
                    <div className="relative text-red-500 font-mono text-sm">{ask.price}</div>
                    <div className="relative text-right text-foreground font-mono text-sm">{ask.amount}</div>
                    <div className="relative text-right text-muted-foreground font-mono text-sm">{ask.total}</div>
                  </div>
                )
              })
            )}
          </div>
        </div>
      </Card>
    </div>
  )
}
