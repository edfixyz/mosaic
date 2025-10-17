import Link from "next/link"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { ArrowRight, TrendingUp, Shield, Zap } from "lucide-react"

export default function HomePage() {
  return (
    <div className="min-h-screen p-8">
      {/* Hero Section */}
      <section className="py-16">
        <div className="max-w-3xl">
          <h1 className="text-6xl font-serif mb-6 text-primary" style={{ fontFamily: "var(--font-playfair)" }}>
            Mosaic
          </h1>
          <p className="text-xl text-muted-foreground mb-4">Professional OTC Trading Desk</p>
          <p className="text-lg text-foreground/80 mb-8 max-w-2xl">
            Execute large-scale digital asset trades with institutional-grade infrastructure and deep liquidity.
          </p>
          <div className="flex gap-4">
            <Button asChild size="lg" className="gap-2">
              <Link href="/markets">
                View Markets
                <ArrowRight className="h-4 w-4" />
              </Link>
            </Button>
            <Button asChild size="lg" variant="outline">
              <Link href="/assets">Browse Assets</Link>
            </Button>
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="py-16">
        <div className="grid md:grid-cols-3 gap-6 max-w-5xl">
          <Card className="p-6 bg-card border-border">
            <div className="h-12 w-12 rounded-lg bg-primary/10 flex items-center justify-center mb-4">
              <TrendingUp className="h-6 w-6 text-primary" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-foreground">Deep Liquidity</h3>
            <p className="text-sm text-muted-foreground">
              Access institutional-grade liquidity for seamless execution of large orders.
            </p>
          </Card>

          <Card className="p-6 bg-card border-border">
            <div className="h-12 w-12 rounded-lg bg-primary/10 flex items-center justify-center mb-4">
              <Shield className="h-6 w-6 text-primary" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-foreground">Secure Settlement</h3>
            <p className="text-sm text-muted-foreground">
              Miden-based settlement ensures cryptographic security for all transactions.
            </p>
          </Card>

          <Card className="p-6 bg-card border-border">
            <div className="h-12 w-12 rounded-lg bg-primary/10 flex items-center justify-center mb-4">
              <Zap className="h-6 w-6 text-primary" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-foreground">Fast Execution</h3>
            <p className="text-sm text-muted-foreground">
              Real-time order matching and execution with minimal slippage.
            </p>
          </Card>
        </div>
      </section>
    </div>
  )
}
