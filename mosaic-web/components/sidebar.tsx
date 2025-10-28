"use client"

import type React from "react"

import Link from "next/link"
import { usePathname, useRouter } from "next/navigation"
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
import { marketStorage, type Market } from "@/lib/market-storage"

export function Sidebar() {
  const pathname = usePathname()
  const router = useRouter()
  const [markets, setMarkets] = useState<Market[]>([])
  const [open, setOpen] = useState(false)
  const [marketUrl, setMarketUrl] = useState("")
  const [marketError, setMarketError] = useState<string | null>(null)
  const [addingMarket, setAddingMarket] = useState(false)
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [marketToDelete, setMarketToDelete] = useState<Market | null>(null)

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

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    setMarketError(null)

    if (!marketUrl.trim()) {
      setMarketError("Enter a market URL.")
      return
    }

    let parsedUrl: URL
    try {
      parsedUrl = new URL(marketUrl.trim())
    } catch {
      setMarketError("Invalid URL. Please enter a fully-qualified URL such as https://host/desk/123.")
      return
    }

    setAddingMarket(true)

    try {
      const response = await fetch(parsedUrl.toString(), {
        headers: {
          Accept: "application/json",
        },
      })

      if (!response.ok) {
        throw new Error(`Server responded with status ${response.status}`)
      }

      const payload = await response.json()

      const expectedFields = ["desk_account", "base_account", "quote_account", "market_url", "owner_account"] as const
      for (const field of expectedFields) {
        if (typeof payload[field] !== "string" || payload[field].trim().length === 0) {
          throw new Error(`Market payload missing '${field}'`)
        }
      }

      const deskAccount: string = payload.desk_account
      const baseAccount: string = payload.base_account
      const quoteAccount: string = payload.quote_account

      const { WebClient, AccountId, Word, Felt } = await import("@demox-labs/miden-sdk")
      const { getOrImportAccount, getDeskInfo } = await import("@/lib/account")

      const client = await WebClient.createClient()

      try {
        await client.syncState()
      } catch (syncError) {
        console.warn("Sync state failed:", syncError)
      }

      const account = await getOrImportAccount(client, AccountId, deskAccount)
      if (!account) {
        throw new Error("Desk account not found on the network.")
      }

      const deskInfo = getDeskInfo(Word, Felt, account)
      if (!deskInfo) {
        throw new Error("Unable to decode desk information from account storage.")
      }

      // if (deskInfo.pair.base.faucet !== baseAccount || deskInfo.pair.quote.faucet !== quoteAccount) {
      //   throw new Error("Desk metadata mismatch between remote response and on-chain data.")
      // }

      marketStorage.saveMarket({
        pair: `${deskInfo.pair.base.symbol}/${deskInfo.pair.quote.symbol}`,
        marketId: deskAccount,
        baseFaucet: baseAccount,
        quoteFaucet: quoteAccount,
        deskUrl: payload.market_url,
      })

      setOpen(false)
      setMarketUrl("")
      router.push(`/desk/${deskAccount}`)
    } catch (error) {
      console.error("Failed to add desk:", error)
      setMarketError(
        error instanceof Error
          ? error.message
          : "Unable to reach the provided Routing URL. Verify the link and try again."
      )
    } finally {
      setAddingMarket(false)
    }
  }

  const handleRemoveMarket = (e: React.MouseEvent, market: Market) => {
    e.preventDefault()
    e.stopPropagation()

    setMarketToDelete(market)
    setDeleteDialogOpen(true)
  }

  const confirmRemoveMarket = () => {
    if (!marketToDelete) return

    const marketPath = `/desk/${marketToDelete.marketId}`
    const isCurrentPage = pathname === marketPath

    marketStorage.removeMarket(marketToDelete.deskUrl)
    const updatedMarkets = marketStorage.getMarkets()
    setMarkets(updatedMarkets)

    if (isCurrentPage) {
      if (updatedMarkets.length > 0) {
        void router.push(`/desk/${updatedMarkets[0].marketId}`)
      } else {
        window.location.href = '/'
      }
    }

    setDeleteDialogOpen(false)
    setMarketToDelete(null)
  }

  return (
    <aside className="fixed left-0 top-[92px] h-[calc(100vh-92px)] w-64 border-r border-border bg-card overflow-y-auto">
      <div className="p-6 flex flex-col h-full" style={{ fontFamily: "var(--font-dm-mono)" }}>
        {/* Markets Section */}
        <div className="flex-1">
          <h3 className="mb-4 text-sm font-semibold text-primary uppercase tracking-wider">Markets</h3>
          <div className="space-y-1">
            {markets.length === 0 ? (
              <p className="text-sm text-muted-foreground px-3 py-2">No markets yet. Visit a market to add it here.</p>
            ) : (
              markets.map((market) => {
                const marketPath = `/desk/${market.marketId}`
                const isActive = pathname === marketPath
                return (
                  <div key={market.deskUrl || `${market.marketId}-${market.pair}`} className="relative group">
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
                      onClick={(e) => handleRemoveMarket(e, market)}
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
                  Provide the desk market URL you want to visit. We will verify the desk details before adding it to your list.
                </DialogDescription>
              </DialogHeader>
              <form onSubmit={handleSubmit} className="space-y-6">
                <div className="space-y-2">
                  <Label htmlFor="marketUrl" className="text-foreground">
                  Market URL
                  </Label>
                  <Input
                    id="marketUrl"
                    placeholder="https://server.com/desk/desk-account"
                    value={marketUrl}
                    onChange={(e) => setMarketUrl(e.target.value)}
                    required
                    className="bg-background border-border text-foreground font-mono text-sm"
                    disabled={addingMarket}
                  />
                  <p className="text-xs text-muted-foreground">
                  Example: https://app.mosaic.xyz/desk/mtst1q...
                  </p>
                  {marketError && (
                    <div className="flex items-center gap-2 text-xs text-destructive">
                      <AlertTriangle className="h-3.5 w-3.5" />
                      <span>{marketError}</span>
                    </div>
                  )}
                </div>

                <Button
                  type="submit"
                  className="w-full bg-primary text-background hover:bg-primary/90"
                  disabled={addingMarket}
                >
                  {addingMarket ? "Validating..." : "Visit Market"}
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
