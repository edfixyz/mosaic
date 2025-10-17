"use client"

import type React from "react"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { useState } from "react"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Plus } from "lucide-react"

const initialMarkets = [
  { pair: "BTC/USDC", path: "/market/BTC/USDC" },
  { pair: "XRP/USD", path: "/market/XRP/USD" },
  { pair: "ETH/USDC", path: "/market/ETH/USDC" },
  { pair: "SOL/USD", path: "/market/SOL/USD" },
  { pair: "BTC/USD", path: "/market/BTC/USD" },
  { pair: "ETH/USD", path: "/market/ETH/USD" },
]

export function Sidebar() {
  const pathname = usePathname()
  const [markets, setMarkets] = useState(initialMarkets)
  const [open, setOpen] = useState(false)
  const [formData, setFormData] = useState({
    baseTicker: "",
    baseAddress: "",
    quoteTicker: "",
    quoteAddress: "",
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    // Validate Miden addresses
    const midenAddressPattern = /^mtst1[a-z0-9]{20,}$/
    if (!midenAddressPattern.test(formData.baseAddress) || !midenAddressPattern.test(formData.quoteAddress)) {
      alert("Invalid Miden address format. Addresses should start with 'mtst1' followed by alphanumeric characters.")
      return
    }

    // Create new market
    const newMarket = {
      pair: `${formData.baseTicker.toUpperCase()}/${formData.quoteTicker.toUpperCase()}`,
      path: `/market/${formData.baseTicker.toUpperCase()}/${formData.quoteTicker.toUpperCase()}`,
    }

    setMarkets([...markets, newMarket])
    setFormData({ baseTicker: "", baseAddress: "", quoteTicker: "", quoteAddress: "" })
    setOpen(false)
  }

  return (
    <aside className="fixed left-0 top-16 h-[calc(100vh-4rem)] w-64 border-r border-border bg-card overflow-y-auto">
      <div className="p-6 flex flex-col h-full" style={{ fontFamily: "var(--font-dm-mono)" }}>
        {/* Markets Section */}
        <div className="flex-1">
          <h3 className="mb-4 text-sm font-semibold text-primary uppercase tracking-wider">Markets</h3>
          <div className="space-y-1">
            {markets.map((market) => {
              const isActive = pathname === market.path
              return (
                <Link
                  key={market.path}
                  href={market.path}
                  className={`block px-3 py-2 rounded-md text-sm transition-colors ${
                    isActive
                      ? "bg-primary/10 text-primary font-medium"
                      : "text-foreground hover:bg-muted hover:text-primary"
                  }`}
                >
                  {market.pair}
                </Link>
              )
            })}
          </div>
        </div>

        <div className="mt-6 pt-6 border-t border-border">
          <Dialog open={open} onOpenChange={setOpen}>
            <DialogTrigger asChild>
              <Button
                variant="outline"
                className="w-full border-primary text-primary hover:bg-primary/10 bg-transparent"
              >
                <Plus className="mr-2 h-4 w-4" />
                Add Market
              </Button>
            </DialogTrigger>
            <DialogContent className="bg-card border-border">
              <DialogHeader>
                <DialogTitle className="text-primary">Create New Market</DialogTitle>
                <DialogDescription className="text-muted-foreground">
                  Enter the ticker and Miden address for both assets
                </DialogDescription>
              </DialogHeader>
              <form onSubmit={handleSubmit} className="space-y-6">
                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="baseTicker" className="text-foreground">
                      Base Ticker
                    </Label>
                    <Input
                      id="baseTicker"
                      placeholder="BTC"
                      value={formData.baseTicker}
                      onChange={(e) => setFormData({ ...formData, baseTicker: e.target.value })}
                      required
                      className="bg-background border-border text-foreground"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="baseAddress" className="text-foreground">
                      Base Miden Address
                    </Label>
                    <Input
                      id="baseAddress"
                      placeholder="mtst1qpdvk6f6ewq8kyqp2tq8w5cchpcqztcwu27"
                      value={formData.baseAddress}
                      onChange={(e) => setFormData({ ...formData, baseAddress: e.target.value })}
                      required
                      className="bg-background border-border text-foreground font-mono text-xs"
                    />
                  </div>
                </div>

                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="quoteTicker" className="text-foreground">
                      Quote Ticker
                    </Label>
                    <Input
                      id="quoteTicker"
                      placeholder="USDC"
                      value={formData.quoteTicker}
                      onChange={(e) => setFormData({ ...formData, quoteTicker: e.target.value })}
                      required
                      className="bg-background border-border text-foreground"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="quoteAddress" className="text-foreground">
                      Quote Miden Address
                    </Label>
                    <Input
                      id="quoteAddress"
                      placeholder="mtst1qpdvk6f6ewq8kyqp2tq8w5cchpcqztcwu27"
                      value={formData.quoteAddress}
                      onChange={(e) => setFormData({ ...formData, quoteAddress: e.target.value })}
                      required
                      className="bg-background border-border text-foreground font-mono text-xs"
                    />
                  </div>
                </div>

                <Button type="submit" className="w-full bg-primary text-background hover:bg-primary/90">
                  Create Market
                </Button>
              </form>
            </DialogContent>
          </Dialog>
        </div>
      </div>
    </aside>
  )
}
