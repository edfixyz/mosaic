"use client"

import type React from "react"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { useState, useEffect } from "react"
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
import { Plus, X, AlertTriangle } from "lucide-react"
import { marketStorage, type Market } from "@/lib/marketStorage"

export function Sidebar() {
  const pathname = usePathname()
  const [markets, setMarkets] = useState<Market[]>([])
  const [open, setOpen] = useState(false)
  const [marketId, setMarketId] = useState("")
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [marketToDelete, setMarketToDelete] = useState<string | null>(null)

  // Load markets from localStorage on mount and listen for changes
  useEffect(() => {
    setMarkets(marketStorage.getMarkets())

    // Listen for storage changes (when markets are added from other tabs/components)
    const handleStorageChange = () => {
      setMarkets(marketStorage.getMarkets())
    }

    window.addEventListener('storage', handleStorageChange)
    // Custom event for same-window updates
    window.addEventListener('marketsUpdated', handleStorageChange)

    return () => {
      window.removeEventListener('storage', handleStorageChange)
      window.removeEventListener('marketsUpdated', handleStorageChange)
    }
  }, [])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    // Validate Miden address format
    const midenAddressPattern = /^[a-z]{4}1[a-z0-9]+$/
    if (!midenAddressPattern.test(marketId)) {
      alert("Invalid Miden address format. Please enter a valid bech32 address.")
      return
    }

    // Navigate to the market page - it will auto-add to the list when loaded
    window.location.href = `/market/${marketId}`
    setMarketId("")
    setOpen(false)
  }

  const handleRemoveMarket = (e: React.MouseEvent, marketId: string) => {
    e.preventDefault()
    e.stopPropagation()

    setMarketToDelete(marketId)
    setDeleteDialogOpen(true)
  }

  const confirmRemoveMarket = () => {
    if (!marketToDelete) return

    // Check if we're currently on this market's page
    const marketPath = `/market/${marketToDelete}`
    const isCurrentPage = pathname === marketPath

    marketStorage.removeMarket(marketToDelete)
    setMarkets(marketStorage.getMarkets())

    // Navigate to home if we're on the removed market's page
    if (isCurrentPage) {
      window.location.href = '/'
    }

    setDeleteDialogOpen(false)
    setMarketToDelete(null)
  }

  return (
    <aside className="fixed left-0 top-16 h-[calc(100vh-4rem)] w-64 border-r border-border bg-card overflow-y-auto">
      <div className="p-6 flex flex-col h-full" style={{ fontFamily: "var(--font-dm-mono)" }}>
        {/* Markets Section */}
        <div className="flex-1">
          <h3 className="mb-4 text-sm font-semibold text-primary uppercase tracking-wider">Markets</h3>
          <div className="space-y-1">
            {markets.length === 0 ? (
              <p className="text-sm text-muted-foreground px-3 py-2">No markets yet. Visit a market to add it here.</p>
            ) : (
              markets.map((market) => {
                const marketPath = `/market/${market.marketId}`
                const isActive = pathname === marketPath
                return (
                  <div key={market.marketId} className="relative group">
                    <Link
                      href={marketPath}
                      className={`flex items-center gap-2 px-3 py-2 pr-8 rounded-md text-sm transition-colors ${
                        isActive
                          ? "bg-primary/10 text-primary font-medium"
                          : "text-foreground hover:bg-muted hover:text-primary"
                      }`}
                    >
                      <span title="Unverified market">
                        <AlertTriangle className="h-3 w-3 text-red-500 shrink-0" />
                      </span>
                      <div className="flex-1 min-w-0">
                        <div className="truncate">{market.pair}</div>
                        <div className="text-[10px] text-muted-foreground font-mono truncate" title={market.marketId}>
                          {market.marketId.slice(0, 12)}...
                        </div>
                      </div>
                    </Link>
                    <button
                      onClick={(e) => handleRemoveMarket(e, market.marketId)}
                      className="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded-md hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-all"
                      title="Remove market"
                    >
                      <X className="h-3.5 w-3.5" />
                    </button>
                  </div>
                )
              })
            )}
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
                <DialogTitle className="text-primary">Visit Market</DialogTitle>
                <DialogDescription className="text-muted-foreground">
                  Enter the market desk account ID (bech32 address)
                </DialogDescription>
              </DialogHeader>
              <form onSubmit={handleSubmit} className="space-y-6">
                <div className="space-y-2">
                  <Label htmlFor="marketId" className="text-foreground">
                    Market Desk ID
                  </Label>
                  <Input
                    id="marketId"
                    placeholder="smaf1qrf9y8pmykxfyqppuehkvds3ffcqqdtepua"
                    value={marketId}
                    onChange={(e) => setMarketId(e.target.value)}
                    required
                    className="bg-background border-border text-foreground font-mono text-sm"
                  />
                  <p className="text-xs text-muted-foreground">
                    The market will be automatically added to your list after loading
                  </p>
                </div>

                <Button type="submit" className="w-full bg-primary text-background hover:bg-primary/90">
                  Visit Market
                </Button>
              </form>
            </DialogContent>
          </Dialog>
        </div>

        {/* Delete Confirmation Dialog */}
        <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
          <DialogContent className="bg-card border-border">
            <DialogHeader>
              <DialogTitle className="text-destructive">Remove Market</DialogTitle>
              <DialogDescription className="text-muted-foreground">
                Are you sure you want to remove this market from your list?
              </DialogDescription>
            </DialogHeader>
            <div className="flex gap-3 justify-end mt-4">
              <Button
                variant="outline"
                onClick={() => setDeleteDialogOpen(false)}
                className="border-border"
              >
                Cancel
              </Button>
              <Button
                onClick={confirmRemoveMarket}
                className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              >
                Remove
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      </div>
    </aside>
  )
}
