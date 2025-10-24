// Market storage utility for persisting markets to localStorage

export interface Market {
  pair: string // e.g., "BTC/USDC"
  marketId: string // bech32 market account ID
  baseFaucet: string // bech32 base faucet address
  quoteFaucet: string // bech32 quote faucet address
  deskUrl: string // full Routing URL used for backend interactions
}

const STORAGE_KEY = 'mosaic_markets'

export const marketStorage = {
  // Get all markets from localStorage
  getMarkets: (): Market[] => {
    if (typeof window === 'undefined') return []

    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      if (!stored) {
        return []
      }

      const parsed = JSON.parse(stored)
      if (!Array.isArray(parsed)) {
        return []
      }

      return parsed.map((market: Partial<Market>) => ({
        pair: market.pair ?? '',
        marketId: market.marketId ?? '',
        baseFaucet: market.baseFaucet ?? '',
        quoteFaucet: market.quoteFaucet ?? '',
        deskUrl: market.deskUrl ?? '',
      })) as Market[]
    } catch (error) {
      console.error('Failed to load markets from storage:', error)
      return []
    }
  },

  // Save a market to localStorage (avoiding duplicates)
  saveMarket: (market: Market): void => {
    if (typeof window === 'undefined') return

    try {
      const markets = marketStorage.getMarkets()

      // Check if market already exists (prefer Routing URL as unique identifier)
      const existingIndex = markets.findIndex(existing => {
        if (existing.deskUrl && market.deskUrl) {
          return existing.deskUrl === market.deskUrl
        }
        // Fallback for legacy entries that might not have a Routing URL persisted yet
        return existing.marketId === market.marketId
      })

      if (existingIndex >= 0) {
        // Update existing market
        markets[existingIndex] = market
      } else {
        // Add new market
        markets.push(market)
      }

      console.log('[marketStorage.saveMarket] writing markets:', markets)

      localStorage.setItem(STORAGE_KEY, JSON.stringify(markets))

      // Dispatch custom event for same-window updates
      window.dispatchEvent(new Event('marketsUpdated'))
    } catch (error) {
      console.error('Failed to save market to storage:', error)
    }
  },

  // Remove a market by Routing URL (routing URL)
  removeMarket: (deskUrl: string): void => {
    if (typeof window === 'undefined') return

    try {
      const markets = marketStorage.getMarkets()
      const filtered = markets.filter(m => m.deskUrl !== deskUrl)
      localStorage.setItem(STORAGE_KEY, JSON.stringify(filtered))

      // Dispatch custom event for same-window updates
      window.dispatchEvent(new Event('marketsUpdated'))
    } catch (error) {
      console.error('Failed to remove market from storage:', error)
    }
  },

  // Clear all markets
  clearMarkets: (): void => {
    if (typeof window === 'undefined') return

    try {
      localStorage.removeItem(STORAGE_KEY)
    } catch (error) {
      console.error('Failed to clear markets from storage:', error)
    }
  },
}
