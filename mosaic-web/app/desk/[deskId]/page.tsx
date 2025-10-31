'use client'

import { OrderBook } from '@/components/order-book'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { TrendingUp, TrendingDown, Loader2 } from 'lucide-react'
import { use, useCallback, useEffect, useMemo, useState } from 'react'
import { getOrImportAccount, getDeskInfo } from '@/lib/account'
import { marketStorage } from '@/lib/market-storage'
import { callMcpTool, OrderPayload, RoleSettings, StoredOrderSummary } from '@/lib/mcp-tool'
import type { ClientAccountInfo, NetworkName } from '@/lib/mcp-tool'

const defaultMarket = { price: 1000, change: '+0.00%', positive: true, volume: '$0' }

type OrderBookEntry = {
  price: string
  amount: string
  total: string
}

type ClientAccount = {
  accountId: string
  network: NetworkName
  name: string | null
}

// Convert bigint quotes to order book format
function formatQuotes(quotes: { amount: bigint, price: bigint }[]): OrderBookEntry[] {
  console.log('Formatting quotes:', quotes.length, 'entries')

  return quotes.map(quote => {
    console.log('Raw quote:', { price: quote.price.toString(), amount: quote.amount.toString() })

    // Convert bigint to string first to avoid precision loss
    const priceStr = quote.price.toString()
    const amountStr = quote.amount.toString()

    // Convert to number
    let price = Number(priceStr)
    let amount = Number(amountStr)

    console.log('Initial conversion:', { price, amount })

    // If values are 0, there might be an issue with the data
    if (price === 0 || amount === 0) {
      console.warn('Zero value detected in quote')
    }

    // Try different decimal adjustments based on magnitude
    // Most crypto uses 8 decimals (satoshis), but let's be flexible
    if (price > 1e10) {
      price = price / 1e8
      console.log('Adjusted price with 8 decimals:', price)
    }
    if (amount > 1e10) {
      amount = amount / 1e8
      console.log('Adjusted amount with 8 decimals:', amount)
    }

    const total = price * amount

    const formatted = {
      price: price.toFixed(8),
      amount: amount.toFixed(8),
      total: total.toFixed(8),
    }

    console.log('Formatted:', formatted)

    return formatted
  })
}

export default function MarketPage({
  params,
}: {
  params: Promise<{ deskId: string }>
}) {
  const { deskId: marketId } = use(params)
  const [base, setBase] = useState<string>('')
  const [quote, setQuote] = useState<string>('')
  const [baseFaucet, setBaseFaucet] = useState<string>('')
  const [quoteFaucet, setQuoteFaucet] = useState<string>('')
  const [bids, setBids] = useState<OrderBookEntry[]>([])
  const [asks, setAsks] = useState<OrderBookEntry[]>([])
  const [lastPrice, setLastPrice] = useState<string>('-')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [deskUrl, setDeskUrl] = useState<string>('')
  const [clientAccounts, setClientAccounts] = useState<ClientAccount[]>([])
  const [accountsLoading, setAccountsLoading] = useState(false)
  const [accountsError, setAccountsError] = useState<string | null>(null)
  const [selectedAccount, setSelectedAccount] = useState<string>('')
  const [orders, setOrders] = useState<StoredOrderSummary[]>([])
  const [ordersLoading, setOrdersLoading] = useState(false)
  const [ordersError, setOrdersError] = useState<string | null>(null)
  const [accessToken, setAccessToken] = useState<string | null>(null)
  const [submittingRequest, setSubmittingRequest] = useState(false)
  const [requestError, setRequestError] = useState<string | null>(null)
  const [roleSettings, setRoleSettings] = useState<RoleSettings | null>(null)
  const [rolesError, setRolesError] = useState<string | null>(null)
  const [liquidityModalOpen, setLiquidityModalOpen] = useState(false)
  const [liquiditySide, setLiquiditySide] = useState<'Buy' | 'Sell'>('Buy')
  const [liquidityAmount, setLiquidityAmount] = useState('')
  const [liquidityPrice, setLiquidityPrice] = useState('')
  const [liquidityError, setLiquidityError] = useState<string | null>(null)
  const [liquiditySubmitting, setLiquiditySubmitting] = useState(false)

  const market = defaultMarket

  const fetchAccessToken = useCallback(async () => {
    try {
      const response = await fetch('/api/auth/token')
      if (!response.ok) {
        return null
      }

      const data = (await response.json()) as { accessToken?: string }
      return data.accessToken ?? null
    } catch (err) {
      console.warn('Failed to fetch access token', err)
      return null
    }
  }, [])

  const loadAccounts = useCallback(
    async (token: string) => {
      setAccountsLoading(true)
      setAccountsError(null)
      try {
        const result = await callMcpTool('list_accounts', {}, token)
        const clients = (result.client_accounts as ClientAccountInfo[])
          .filter((acct) => acct.account_type === 'Client')
          .map<ClientAccount>((acct) => ({
            accountId: acct.account_id,
            network: acct.network === 'Localnet' ? 'Localnet' : 'Testnet',
            name: acct.name ?? null,
          }))

        setClientAccounts(clients)
        if (clients.length > 0) {
          setSelectedAccount((current) =>
            current && clients.some((account) => account.accountId === current)
              ? current
              : clients[0].accountId
          )
        } else {
          setSelectedAccount('')
        }
      } catch (err) {
        console.error('Failed to load client accounts', err)
        setAccountsError('Unable to load client accounts')
        setClientAccounts([])
        setSelectedAccount('')
      } finally {
        setAccountsLoading(false)
      }
    },
    []
  )

  const loadOrders = useCallback(
    async (token: string) => {
      setOrdersLoading(true)
      setOrdersError(null)
      try {
        const result = await callMcpTool('list_orders', {}, token)
        setOrders(result)
      } catch (err) {
        console.error('Failed to load orders', err)
        setOrdersError('Unable to load orders')
        setOrders([])
      } finally {
        setOrdersLoading(false)
      }
    },
    []
  )

  const loadRoleSettings = useCallback(
    async (token: string) => {
      setRolesError(null)
      try {
        const settings = await callMcpTool('get_role_settings', {}, token)
        setRoleSettings(settings)
      } catch (err) {
        console.error('Failed to load role settings', err)
        setRolesError('Unable to load role settings')
        setRoleSettings(null)
      }
    },
    []
  )

  const [requestModalOpen, setRequestModalOpen] = useState(false)
  const [requestSide, setRequestSide] = useState<'Buy' | 'Sell'>('Buy')
  const [requestAmount, setRequestAmount] = useState('')
  const [settlementMethod, setSettlementMethod] = useState('Onchain')

  const parseOrderSummary = useCallback((order: StoredOrderSummary) => {
    let variant = order.order_type
    let payload: Record<string, unknown> | null = null

    try {
      const parsed = JSON.parse(order.order_json) as unknown
      if (typeof parsed === 'string') {
        variant = parsed
      } else if (parsed && typeof parsed === 'object') {
        const [key] = Object.keys(parsed as Record<string, unknown>)
        if (key) {
          variant = key
          payload = (parsed as Record<string, unknown>)[key] as Record<string, unknown>
        }
      }
    } catch (err) {
      console.warn('Failed to parse order payload', err)
    }

    const payloadMap = payload as Record<string, unknown> | null

    let payloadMarket: string | undefined
    if (payloadMap) {
      const rawMarket = payloadMap['market']
      if (typeof rawMarket === 'string') {
        payloadMarket = rawMarket
      } else if (rawMarket && typeof rawMarket === 'object') {
        const marketObj = rawMarket as Record<string, unknown>
        const baseObj = marketObj['base'] as Record<string, unknown> | undefined
        const quoteObj = marketObj['quote'] as Record<string, unknown> | undefined
        const baseCode =
          baseObj && typeof baseObj['code'] === 'string' ? (baseObj['code'] as string) : undefined
        const quoteCode =
          quoteObj && typeof quoteObj['code'] === 'string'
            ? (quoteObj['code'] as string)
            : undefined
        if (baseCode && quoteCode) {
          payloadMarket = `${baseCode}/${quoteCode}`
        }
      }
    }

    const payloadSide =
      payloadMap && typeof payloadMap['side'] === 'string'
        ? (payloadMap['side'] as string)
        : undefined
    const payloadAmount =
      payloadMap && typeof payloadMap['amount'] === 'number'
        ? (payloadMap['amount'] as number)
        : undefined
    const payloadPrice =
      payloadMap && typeof payloadMap['price'] === 'number'
        ? (payloadMap['price'] as number)
        : undefined

    return {
      variant,
      payload,
      market: payloadMarket,
      side: payloadSide,
      amount: payloadAmount,
      price: payloadPrice,
    }
  }, [])

  const openRequestModal = (side: 'Buy' | 'Sell') => {
    setRequestSide(side)
    setRequestError(null)
    setRequestModalOpen(true)
  }

  const resetRequestState = () => {
    setRequestSide('Buy')
    setRequestAmount('')
    setSettlementMethod('Onchain')
    setRequestError(null)
  }

  const handleCloseModal = (open: boolean) => {
    if (!open) {
      setRequestModalOpen(false)
      resetRequestState()
    } else {
      setRequestModalOpen(true)
    }
  }

  const openLiquidityModal = (side: 'Buy' | 'Sell') => {
    setLiquiditySide(side)
    setLiquidityAmount('')
    setLiquidityPrice('')
    setLiquidityError(null)
    setLiquidityModalOpen(true)
  }

  const resetLiquidityState = () => {
    setLiquiditySide('Buy')
    setLiquidityAmount('')
    setLiquidityPrice('')
    setLiquidityError(null)
  }

  const handleCloseLiquidityModal = (open: boolean) => {
    if (!open) {
      setLiquidityModalOpen(false)
      resetLiquidityState()
    } else {
      setLiquidityModalOpen(true)
    }
  }

  const handleSubmitRequest = async () => {
    setRequestError(null)

    if (!selectedAccount) {
      setRequestError('Select an account to continue')
      return
    }

    const targetAccount = clientAccounts.find((account) => account.accountId === selectedAccount)
    if (!targetAccount) {
      setRequestError('The selected account is no longer available')
      return
    }

    const amountValue = Number(requestAmount)
    if (!Number.isFinite(amountValue) || amountValue <= 0) {
      setRequestError('Amount must be greater than zero')
      return
    }

    const roundedAmount = Math.round(amountValue)
    const orderPayload: OrderPayload = {
      QuoteRequest: {
        market: base && quote ? `${base}/${quote}` : marketId,
        uuid: Math.floor(Math.random() * 1_000_000_000_000),
        side: requestSide === 'Buy' ? 'BUY' : 'SELL',
        amount: roundedAmount,
      },
    }

    setSubmittingRequest(true)

    let token = accessToken
    try {
      if (!token) {
        token = await fetchAccessToken()
        if (!token) {
          throw new Error('You must be logged in to submit an order')
        }
        setAccessToken(token)
      }

      const createOrderResponse = await callMcpTool(
        'create_order',
        {
          network: targetAccount.network,
          account_id: targetAccount.accountId,
          order: orderPayload,
          commit: true,
        },
        token
      )

      if (!createOrderResponse.success) {
        throw new Error('Failed to create desk note')
      }

      await postNoteToDesk(createOrderResponse.note)

      await loadOrders(token)
      setRequestModalOpen(false)
      resetRequestState()
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to submit order'
      setRequestError(message)

      if (token) {
        await loadOrders(token).catch((loadErr) => {
          console.warn('Failed to refresh orders after error', loadErr)
        })
      }
    } finally {
      setSubmittingRequest(false)
    }
  }

  const handleSubmitLiquidity = async () => {
    setLiquidityError(null)

    if (!selectedAccount) {
      setLiquidityError('Select an account to continue')
      return
    }

    const targetAccount = clientAccounts.find((account) => account.accountId === selectedAccount)
    if (!targetAccount) {
      setLiquidityError('The selected account is no longer available')
      return
    }

    const amountValue = Number(liquidityAmount)
    if (!Number.isFinite(amountValue) || amountValue <= 0) {
      setLiquidityError('Amount must be greater than zero')
      return
    }

    const priceValue = Number(liquidityPrice)
    if (!Number.isFinite(priceValue) || priceValue <= 0) {
      setLiquidityError('Price must be greater than zero')
      return
    }

    const roundedAmount = Math.round(amountValue)
    const roundedPrice = Math.round(priceValue)

    const orderPayload: OrderPayload = {
      LiquidityOffer: {
        market: base && quote ? `${base}/${quote}` : marketId,
        uuid: Math.floor(Math.random() * 1_000_000_000_000),
        side: liquiditySide === 'Buy' ? 'BUY' : 'SELL',
        amount: roundedAmount,
        price: roundedPrice,
      },
    }

    setLiquiditySubmitting(true)

    let token = accessToken
    try {
      if (!token) {
        token = await fetchAccessToken()
        if (!token) {
          throw new Error('You must be logged in to submit an offer')
        }
        setAccessToken(token)
      }

      const createOrderResponse = await callMcpTool(
        'create_order',
        {
          network: targetAccount.network,
          account_id: targetAccount.accountId,
          order: orderPayload,
          commit: true,
        },
        token
      )

      if (!createOrderResponse.success) {
        throw new Error('Failed to create desk note')
      }

      await postNoteToDesk(createOrderResponse.note)

      await loadOrders(token)
      setLiquidityModalOpen(false)
      resetLiquidityState()
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to submit liquidity offer'
      setLiquidityError(message)

      if (token) {
        await loadOrders(token).catch((loadErr) => {
          console.warn('Failed to refresh orders after error', loadErr)
        })
      }
    } finally {
      setLiquiditySubmitting(false)
    }
  }

  useEffect(() => {
    const loadMarketInfo = async () => {
      try {
        let resolvedDeskUrl = ''

        if (typeof window !== 'undefined') {
          const storedMarkets = marketStorage.getMarkets()
          const storedMarket = storedMarkets.find((entry) => entry.marketId === marketId)
          if (storedMarket?.deskUrl) {
            resolvedDeskUrl = storedMarket.deskUrl
          }
        }

        if (!resolvedDeskUrl) {
          const serverBase = process.env.NEXT_PUBLIC_MOSAIC_SERVER?.trim().replace(/\/$/, '')
          if (serverBase) {
            resolvedDeskUrl = `${serverBase}/desk/${marketId}`
          } else if (typeof window !== 'undefined') {
            resolvedDeskUrl = `${window.location.origin.replace(/\/$/, '')}/desk/${marketId}`
          }
        }

        setDeskUrl(resolvedDeskUrl)

        // Dynamically import the SDK
        const { WebClient, AccountId, Word, Felt } = await import('@demox-labs/miden-sdk')

        // Initialize client
        const client = await WebClient.createClient()

        // Sync state (may fail for new clients)
        try {
          await client.syncState()
        } catch (syncError) {
          console.warn('Sync state failed:', syncError)
        }

        // Get or import the account
        const account = await getOrImportAccount(client, AccountId, marketId)

        if (!account) {
          setError('Account not found on the network')
          return
        }

        // Get desk info (base and quote symbols)
        const deskInfo = getDeskInfo(Word, Felt, account)

        if (deskInfo) {
          setBase(deskInfo.pair.base.symbol)
          setQuote(deskInfo.pair.quote.symbol)
          setBaseFaucet(deskInfo.pair.base.faucet)
          setQuoteFaucet(deskInfo.pair.quote.faucet)

          // Format and set order book data
          // Sell quotes are "asks" (people selling base for quote)
          const formattedAsks = formatQuotes(deskInfo.quotes.sell)
          setAsks(formattedAsks)
          // Buy quotes are "bids" (people buying base with quote)
          const formattedBids = formatQuotes(deskInfo.quotes.buy)
          setBids(formattedBids)

          // Calculate last price (use first ask if available, otherwise first bid)
          if (formattedAsks.length > 0) {
            setLastPrice(formattedAsks[0].price)
          } else if (formattedBids.length > 0) {
            setLastPrice(formattedBids[0].price)
          } else {
            setLastPrice('-')
          }

          // Save market to localStorage
          marketStorage.saveMarket({
            pair: `${deskInfo.pair.base.symbol}/${deskInfo.pair.quote.symbol}`,
            marketId,
            baseFaucet: deskInfo.pair.base.faucet,
            quoteFaucet: deskInfo.pair.quote.faucet,
            deskUrl: resolvedDeskUrl,
          })
        } else {
          setError('Market not found or invalid market data')
        }
      } catch (error) {
        console.error('Failed to load market info:', error)
        const errorMessage = error instanceof Error ? error.message : 'Failed to load market'
        setError(errorMessage)
      } finally {
        setLoading(false)
      }
    }

    loadMarketInfo()
  }, [marketId])

  useEffect(() => {
    const initialise = async () => {
      const token = await fetchAccessToken()
      setAccessToken(token)

      if (!token) {
        setClientAccounts([])
        setOrders([])
        setSelectedAccount('')
        setRoleSettings(null)
        return
      }

      await loadAccounts(token)
      await loadOrders(token)
      await loadRoleSettings(token)
    }

    void initialise()
  }, [fetchAccessToken, loadAccounts, loadOrders, loadRoleSettings, marketId])

  const marketSymbol = useMemo(() => {
    if (base && quote) {
      return `${base}/${quote}`
    }
    return null
  }, [base, quote])

  const ordersForMarket = useMemo(() => {
    return orders
      .map((order) => {
        const details = parseOrderSummary(order)
        return { order, details }
      })
      .filter(({ details }) => {
        if (!marketSymbol) {
          return false
        }
        if (!details.market) {
          return false
        }
        return details.market === marketSymbol
      })
  }, [orders, marketSymbol, parseOrderSummary])

  const accountNameById = useMemo(() => {
    const map = new Map<string, string>()
    clientAccounts.forEach((account) => {
      if (account.accountId) {
        map.set(account.accountId, account.name ?? account.accountId)
      }
    })
    return map
  }, [clientAccounts])

  const postNoteToDesk = useCallback(
    async (note: unknown) => {
      if (!note) {
        throw new Error('Missing desk note payload')
      }

      if (!deskUrl) {
        throw new Error('Routing URL is not available for this market')
      }

      const sanitized = deskUrl.endsWith('/') ? deskUrl.slice(0, -1) : deskUrl
      const endpoint = `${sanitized}/note`

      const response = await fetch(endpoint, {
        method: 'POST',
        credentials: 'omit',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ note }),
      })

      if (!response.ok) {
        const message = await response.text().catch(() => '')
        throw new Error(message || `Desk note submission failed (${response.status})`)
      }
    },
    [deskUrl]
  )

  const canRequestQuote = roleSettings?.is_client ?? false
  const canOfferLiquidity = roleSettings?.is_liquidity_provider ?? false

  return (
    <div className="min-h-screen p-8">
      {/* Market Header */}
      <div className="mb-8">
        {loading ? (
          <Card className="p-8 bg-card border-border">
            <div className="flex flex-col items-center justify-center gap-4">
              <Loader2 className="h-8 w-8 animate-spin text-primary" />
              <p className="text-lg text-muted-foreground">Loading market data...</p>
            </div>
          </Card>
        ) : error ? (
          <Card className="p-8 bg-card border-destructive">
            <div className="text-center">
              <h1 className="text-2xl font-serif text-destructive mb-2" style={{ fontFamily: "var(--font-playfair)" }}>
                Market Not Found
              </h1>
              <p className="text-muted-foreground mb-4">{error}</p>
              <p className="text-sm text-muted-foreground font-mono">
                Market ID: {marketId}
              </p>
            </div>
          </Card>
        ) : (
          <>
            <div className="mb-4">
              <div className="flex items-center gap-3 mb-2">
                <h1 className="text-4xl font-serif text-primary" style={{ fontFamily: "var(--font-playfair)" }}>
                  <span className="relative inline-block group">
                    <span className="cursor-help">{base}</span>
                    {baseFaucet && (
                      <span className="absolute left-1/2 -translate-x-1/2 top-full mt-2 px-3 py-2 bg-popover text-popover-foreground text-xs font-mono rounded-md border border-border shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all whitespace-nowrap z-10">
                        {baseFaucet}
                      </span>
                    )}
                  </span>
                  /
                  <span className="relative inline-block group">
                    <span className="cursor-help">{quote}</span>
                    {quoteFaucet && (
                      <span className="absolute left-1/2 -translate-x-1/2 top-full mt-2 px-3 py-2 bg-popover text-popover-foreground text-xs font-mono rounded-md border border-border shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all whitespace-nowrap z-10">
                        {quoteFaucet}
                      </span>
                    )}
                  </span>
                </h1>
                <Badge variant="outline" className="text-sm">
                  OTC Market
                </Badge>
                <Badge variant="outline" className="text-sm text-red-500">
                  Unverified
                </Badge>
              </div>
              <a
                href={`https://testnet.midenscan.com/account/${marketId}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-muted-foreground hover:text-primary transition-colors font-mono"
              >
                {marketId} ↗
              </a>
              <div className="mt-2 text-sm text-muted-foreground font-mono">
                Routing URL:{' '}
                {deskUrl ? (
                  <a
                    href={deskUrl}
                    className="text-primary underline-offset-2 hover:underline break-all"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    {deskUrl}
                  </a>
                ) : (
                  <span className="text-muted-foreground/80">Loading…</span>
                )}
              </div>
            </div>

            <Card className="p-6 bg-card border-border">
              <div className="grid md:grid-cols-4 gap-6">
                <div>
                  <p className="text-sm text-muted-foreground mb-1">Last Price</p>
                  <p className="text-3xl font-semibold text-foreground">{lastPrice === '-' ? '-' : `$${lastPrice}`}</p>
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
          </>
        )}
      </div>

      {!loading && !error && (
        <Card className={`mb-8 bg-card border-border ${canRequestQuote || canOfferLiquidity ? 'p-6' : 'p-4'}`}>
          <div className={`flex items-center justify-between ${canRequestQuote || canOfferLiquidity ? 'mb-6' : 'mb-3'}`}>
            <h2 className="text-xl font-semibold text-foreground">Quotes &amp; Orders</h2>
            {ordersLoading && <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />}
          </div>
          {(!accessToken || clientAccounts.length === 0) && !canRequestQuote && !canOfferLiquidity ? (
            <p className="text-sm text-muted-foreground">Connect with Mosaic to place orders.</p>
          ) : rolesError ? (
            <p className="text-sm text-destructive">{rolesError}</p>
          ) : ordersError ? (
            <p className="text-sm text-destructive">{ordersError}</p>
          ) : ordersForMarket.length === 0 ? (
            <p className="text-sm text-muted-foreground">No orders yet.</p>
          ) : (
            <div className="space-y-2">
              <div className="grid grid-cols-1 gap-2 text-xs font-medium text-muted-foreground md:grid-cols-6">
                <span>Type</span>
                <span>Side</span>
                <span>Amount</span>
                <span>Status</span>
                <span>Account</span>
                <span>Created</span>
              </div>
              <div className="divide-y divide-border">
                {ordersForMarket.map(({ order, details }) => {
                  const sideLabel = details.side
                    ? details.side.toString().toUpperCase()
                    : '-'
                  const amountLabel =
                    typeof details.amount === 'number'
                      ? details.amount.toLocaleString()
                      : '-'
                  const typeLabel = details.variant.replace(/([A-Z])/g, ' $1').trim()
                  const stagePart = order.stage ? order.stage.toString() : ''
                  const statusPart = order.status ? order.status.toString() : ''
                  const statusDisplay = [stagePart, statusPart]
                    .filter((part) => part.length > 0)
                    .join(' / ')
                  const stageLabel = statusDisplay
                    ? statusDisplay.replace(/\b\w/g, (char) => char.toUpperCase())
                    : '-'
                  const createdLabel = order.created_at
                    ? new Date(order.created_at).toLocaleString()
                    : '-'
                  const accountLabel = accountNameById.get(order.account) ?? order.account

                  return (
                    <div
                      key={`${order.uuid}-${order.account}-${order.stage}`}
                      className="grid grid-cols-1 gap-2 py-3 text-sm md:grid-cols-6"
                    >
                      <span className="font-mono text-foreground">{typeLabel}</span>
                      <span className="uppercase text-primary">{sideLabel || '-'}</span>
                      <span className="text-foreground">{amountLabel}</span>
                      <span className="text-foreground capitalize">{stageLabel}</span>
                      <span className="truncate text-muted-foreground">{accountLabel}</span>
                      <span className="text-muted-foreground">{createdLabel}</span>
                    </div>
                  )
                })}
              </div>
            </div>
          )}
        </Card>
      )}

      {/* Order Book */}
      {!loading && !error && (
        <OrderBook
          bids={bids}
          asks={asks}
          baseAsset={base}
          quoteAsset={quote}
          canRequestQuote={canRequestQuote}
          canOfferLiquidity={canOfferLiquidity}
          onRequestQuote={openRequestModal}
          onOfferLiquidity={openLiquidityModal}
        />
      )}

      <Dialog open={requestModalOpen} onOpenChange={handleCloseModal}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Request {requestSide} Quote</DialogTitle>
            <DialogDescription>
              Submit a {requestSide.toLowerCase()} request for {base}/{quote}.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="grid gap-2">
              <Label htmlFor="request-account">Client Account</Label>
              <Select
                value={selectedAccount}
                onValueChange={setSelectedAccount}
                disabled={accountsLoading || clientAccounts.length === 0 || submittingRequest}
              >
                <SelectTrigger id="request-account">
                  <SelectValue
                    placeholder={
                      accountsLoading
                        ? 'Loading accounts...'
                        : clientAccounts.length === 0
                          ? 'No client accounts found'
                          : 'Select account'
                    }
                  />
                </SelectTrigger>
                <SelectContent>
                  {clientAccounts.map((account) => (
                    <SelectItem key={account.accountId} value={account.accountId}>
                      {account.name ?? account.accountId}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {accountsError && (
                <p className="text-sm text-destructive">{accountsError}</p>
              )}
            </div>
            <div className="grid gap-2">
              <Label>Side</Label>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant={requestSide === 'Buy' ? 'default' : 'outline'}
                  onClick={() => setRequestSide('Buy')}
                  disabled={submittingRequest}
                  className="flex-1"
                >
                  Buy
                </Button>
                <Button
                  type="button"
                  variant={requestSide === 'Sell' ? 'default' : 'outline'}
                  onClick={() => setRequestSide('Sell')}
                  disabled={submittingRequest}
                  className="flex-1"
                >
                  Sell
                </Button>
              </div>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="request-amount">Amount ({base || 'Base'})</Label>
              <Input
                id="request-amount"
                type="number"
                min="0"
                placeholder="0.0"
                value={requestAmount}
                onChange={(event) => setRequestAmount(event.target.value)}
                disabled={submittingRequest}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="settlement-method">Settlement</Label>
              <Select
                value={settlementMethod}
                onValueChange={setSettlementMethod}
                disabled={submittingRequest}
              >
                <SelectTrigger id="settlement-method">
                  <SelectValue placeholder="Select settlement" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Onchain">Onchain</SelectItem>
                </SelectContent>
              </Select>
            </div>
            {requestError && (
              <p className="text-sm text-destructive">{requestError}</p>
            )}
          </div>
          <DialogFooter className="flex gap-2">
            <Button
              variant="outline"
              onClick={() => handleCloseModal(false)}
              disabled={submittingRequest}
            >
              Cancel
            </Button>
            <Button
              onClick={handleSubmitRequest}
              disabled={
                submittingRequest ||
                !selectedAccount ||
                !requestAmount ||
                Number(requestAmount) <= 0
              }
            >
              {submittingRequest && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Submit Request
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={liquidityModalOpen} onOpenChange={handleCloseLiquidityModal}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Offer Liquidity ({liquiditySide})</DialogTitle>
            <DialogDescription>
              Provide a liquidity offer for {base}/{quote}. Offers require an account and a price.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="grid gap-2">
              <Label htmlFor="liquidity-account">Client Account</Label>
              <Select
                value={selectedAccount}
                onValueChange={setSelectedAccount}
                disabled={accountsLoading || clientAccounts.length === 0 || liquiditySubmitting}
              >
                <SelectTrigger id="liquidity-account">
                  <SelectValue
                    placeholder={
                      accountsLoading
                        ? 'Loading accounts...'
                        : clientAccounts.length === 0
                          ? 'No client accounts found'
                          : 'Select account'
                    }
                  />
                </SelectTrigger>
                <SelectContent>
                  {clientAccounts.map((account) => (
                    <SelectItem key={account.accountId} value={account.accountId}>
                      {account.name ?? account.accountId}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-2">
              <Label>Side</Label>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant={liquiditySide === 'Buy' ? 'default' : 'outline'}
                  onClick={() => setLiquiditySide('Buy')}
                  disabled={liquiditySubmitting}
                  className="flex-1"
                >
                  Buy
                </Button>
                <Button
                  type="button"
                  variant={liquiditySide === 'Sell' ? 'default' : 'outline'}
                  onClick={() => setLiquiditySide('Sell')}
                  disabled={liquiditySubmitting}
                  className="flex-1"
                >
                  Sell
                </Button>
              </div>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="liquidity-amount">Amount ({base || 'Base'})</Label>
              <Input
                id="liquidity-amount"
                type="number"
                min="0"
                placeholder="0.0"
                value={liquidityAmount}
                onChange={(event) => setLiquidityAmount(event.target.value)}
                disabled={liquiditySubmitting}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="liquidity-price">Price ({quote || 'Quote'})</Label>
              <Input
                id="liquidity-price"
                type="number"
                min="0"
                placeholder="0.0"
                value={liquidityPrice}
                onChange={(event) => setLiquidityPrice(event.target.value)}
                disabled={liquiditySubmitting}
              />
            </div>
            {liquidityError && (
              <p className="text-sm text-destructive">{liquidityError}</p>
            )}
          </div>
          <DialogFooter className="flex gap-2">
            <Button
              variant="outline"
              onClick={() => handleCloseLiquidityModal(false)}
              disabled={liquiditySubmitting}
            >
              Cancel
            </Button>
            <Button
              onClick={handleSubmitLiquidity}
              disabled={
                liquiditySubmitting ||
                !selectedAccount ||
                !liquidityAmount ||
                Number(liquidityAmount) <= 0 ||
                !liquidityPrice ||
                Number(liquidityPrice) <= 0
              }
            >
              {liquiditySubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Submit Offer
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
