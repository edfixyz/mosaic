import Link from "next/link"
import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { ArrowRight, TrendingUp, TrendingDown } from "lucide-react"

const markets = [
  {
    pair: "BTC/USDC",
    base: "BTC",
    quote: "USDC",
    price: "$94,234.50",
    change: "+2.34%",
    volume: "$45.2M",
    positive: true,
  },
  {
    pair: "XRP/USD",
    base: "XRP",
    quote: "USD",
    price: "$2.18",
    change: "+5.67%",
    volume: "$12.8M",
    positive: true,
  },
  {
    pair: "ETH/USDC",
    base: "ETH",
    quote: "USDC",
    price: "$3,456.78",
    change: "-1.23%",
    volume: "$38.9M",
    positive: false,
  },
  {
    pair: "SOL/USD",
    base: "SOL",
    quote: "USD",
    price: "$145.32",
    change: "+8.91%",
    volume: "$23.4M",
    positive: true,
  },
  {
    pair: "BTC/USDT",
    base: "BTC",
    quote: "USDT",
    price: "$94,189.00",
    change: "+2.28%",
    volume: "$52.1M",
    positive: true,
  },
  {
    pair: "ETH/USD",
    base: "ETH",
    quote: "USD",
    price: "$3,461.23",
    change: "-1.15%",
    volume: "$41.7M",
    positive: false,
  },
]

export default function MarketsPage() {
  return (
    <div className="min-h-screen p-8">
      <div className="mb-12 text-center space-y-3">
        <h1 className="text-4xl font-serif text-primary" style={{ fontFamily: "var(--font-playfair)" }}>
          Markets
        </h1>
        <p className="text-3xl font-semibold text-foreground/80">Coming Soon</p>
        <p className="max-w-2xl mx-auto text-base text-muted-foreground">
          We&apos;re crafting a new way to explore Mosaic&apos;s OTC markets. Check back soon for curated trading
          opportunities and liquidity programs tailored for professional participants.
        </p>
      </div>
      <div
        className="grid md:grid-cols-2 gap-4 blur-sm opacity-60 pointer-events-none select-none"
        style={{ fontFamily: "var(--font-dm-mono)" }}
      >
        {markets.map((market) => (
          <Card key={market.pair} className="p-6 bg-card border-border">
            <div className="flex items-start justify-between mb-4">
              <div>
                <h3 className="text-2xl font-semibold text-foreground mb-1">{market.pair}</h3>
                <p className="text-sm text-muted-foreground">
                  {market.base} / {market.quote}
                </p>
              </div>
              <div className={`flex items-center gap-1 ${market.positive ? "text-green-500" : "text-red-500"}`}>
                {market.positive ? <TrendingUp className="h-4 w-4" /> : <TrendingDown className="h-4 w-4" />}
                <span className="text-sm font-medium">{market.change}</span>
              </div>
            </div>

            <div className="flex items-end justify-between">
              <div>
                <div className="text-3xl font-semibold text-foreground mb-1">{market.price}</div>
                <div className="text-sm text-muted-foreground">24h Volume: {market.volume}</div>
              </div>
              <Button asChild variant="outline" size="sm" className="gap-2 bg-transparent">
                <Link href={`/market/${market.base}/${market.quote}`}>
                  View Book
                  <ArrowRight className="h-3 w-3" />
                </Link>
              </Button>
            </div>
          </Card>
        ))}
      </div>
    </div>
  )
}
