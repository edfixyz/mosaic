import { Card } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Coins } from "lucide-react"

const assets = [
  {
    ticker: "BTC",
    name: "Bitcoin",
    midenAddress: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
    price: "$94,234.50",
    change: "+2.34%",
    positive: true,
  },
  {
    ticker: "XRP",
    name: "Ripple",
    midenAddress: "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063",
    price: "$2.18",
    change: "+5.67%",
    positive: true,
  },
  {
    ticker: "USDC",
    name: "USD Coin",
    midenAddress: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
    price: "$1.00",
    change: "+0.01%",
    positive: true,
  },
  {
    ticker: "ETH",
    name: "Ethereum",
    midenAddress: "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619",
    price: "$3,456.78",
    change: "-1.23%",
    positive: false,
  },
  {
    ticker: "SOL",
    name: "Solana",
    midenAddress: "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6",
    price: "$145.32",
    change: "+8.91%",
    positive: true,
  },
  {
    ticker: "USDT",
    name: "Tether",
    midenAddress: "0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
    price: "$1.00",
    change: "0.00%",
    positive: true,
  },
]

export default function AssetsPage() {
  return (
    <div className="min-h-screen p-8">
      <div className="mb-8">
        <h1 className="text-4xl font-serif mb-2 text-primary" style={{ fontFamily: "var(--font-playfair)" }}>
          Assets
        </h1>
        <p className="text-muted-foreground">Digital assets available for OTC trading on Mosaic</p>
      </div>

      <div className="grid gap-4" style={{ fontFamily: "var(--font-dm-mono)" }}>
        {assets.map((asset) => (
          <Card
            key={asset.ticker}
            id={asset.ticker}
            className="p-6 bg-card border-border hover:border-primary/50 transition-colors"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <div className="h-12 w-12 rounded-full bg-primary/10 flex items-center justify-center">
                  <Coins className="h-6 w-6 text-primary" />
                </div>
                <div>
                  <div className="flex items-center gap-3 mb-1">
                    <h3 className="text-xl font-semibold text-foreground">{asset.ticker}</h3>
                    <Badge variant="outline" className="text-xs">
                      {asset.name}
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">Miden: {asset.midenAddress}</p>
                </div>
              </div>
              <div className="text-right">
                <div className="text-2xl font-semibold text-foreground mb-1">{asset.price}</div>
                <div className={`text-sm font-medium ${asset.positive ? "text-green-500" : "text-red-500"}`}>
                  {asset.change}
                </div>
              </div>
            </div>
          </Card>
        ))}
      </div>
    </div>
  )
}
