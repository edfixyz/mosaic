// Market storage utility for persisting markets to localStorage

export interface Market {
  pair: string // e.g., "BTC/USDC"
  marketId: string // bech32 market account ID
  baseFaucet: string // bech32 base faucet address
  quoteFaucet: string // bech32 quote faucet address
}

const STORAGE_KEY = 'mosaic_markets'

export const marketStorage = {
  // Get all markets from localStorage
  getMarkets: (): Market[] => {
    if (typeof window === 'undefined') return []

    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      return stored ? JSON.parse(stored) : []
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

      // Check if market already exists
      const existingIndex = markets.findIndex(m => m.marketId === market.marketId)

      if (existingIndex >= 0) {
        // Update existing market
        markets[existingIndex] = market
      } else {
        // Add new market
        markets.push(market)
      }

      localStorage.setItem(STORAGE_KEY, JSON.stringify(markets))

      // Dispatch custom event for same-window updates
      window.dispatchEvent(new Event('marketsUpdated'))
    } catch (error) {
      console.error('Failed to save market to storage:', error)
    }
  },

  // Remove a market by marketId
  removeMarket: (marketId: string): void => {
    if (typeof window === 'undefined') return

    try {
      const markets = marketStorage.getMarkets()
      const filtered = markets.filter(m => m.marketId !== marketId)
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
