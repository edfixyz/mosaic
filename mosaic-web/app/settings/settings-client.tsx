'use client'

import Link from 'next/link'
import { useCallback, useEffect, useMemo, useState } from 'react'
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
  DialogTrigger,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import {
  AssetSummary,
  RoleSettings,
  callMcpTool,
  NetworkName,
  DeskAccountInfo,
  type CreateAccountOrderResponse,
} from '@/lib/mcp-tool'
import { marketStorage } from '@/lib/market-storage'
import { formatAssetSupply } from '@/lib/asset-format'
import { Coins, AlertCircle, Plus, Loader2, Wallet, Briefcase, Copy } from 'lucide-react'
import clsx from 'clsx'

type ClientAccount = {
  accountId: string
  network: NetworkName
  name: string | null
}

type DeskAccount = {
  accountId: string
  network: NetworkName
  market: DeskAccountInfo['market']
  ownerAccount: string
  ownerName: string | null
  marketUrl: string
}

export function SettingsClient() {
  const [clientAccounts, setClientAccounts] = useState<ClientAccount[] | null>(null)
  const [deskAccounts, setDeskAccounts] = useState<DeskAccount[] | null>(null)
  const [accountsLoading, setAccountsLoading] = useState(true)
  const [accountsError, setAccountsError] = useState(false)
  const [accountsHasAccess, setAccountsHasAccess] = useState(false)

  const [createAccountModalOpen, setCreateAccountModalOpen] = useState(false)
  const [creatingAccount, setCreatingAccount] = useState(false)
  const [accountName, setAccountName] = useState('')
  const [accountNetwork, setAccountNetwork] = useState<NetworkName>('Testnet')
  const [createDeskModalOpen, setCreateDeskModalOpen] = useState(false)
  const [creatingDesk, setCreatingDesk] = useState(false)
  const [deskBaseAccount, setDeskBaseAccount] = useState('')
  const [deskQuoteAccount, setDeskQuoteAccount] = useState('')
  const [deskOwnerAccount, setDeskOwnerAccount] = useState('')
  const [deskError, setDeskError] = useState<string | null>(null)

  const [assets, setAssets] = useState<AssetSummary[] | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(false)
  const [hasAccess, setHasAccess] = useState(false)

  const [createModalOpen, setCreateModalOpen] = useState(false)
  const [addModalOpen, setAddModalOpen] = useState(false)

  const [creating, setCreating] = useState(false)
  const [adding, setAdding] = useState(false)

  const [tokenSymbol, setTokenSymbol] = useState('')
  const [decimals, setDecimals] = useState('8')
  const [maxSupply, setMaxSupply] = useState('')
  const [network, setNetwork] = useState('Testnet')

  const [addSymbol, setAddSymbol] = useState('')
  const [addAccount, setAddAccount] = useState('')
  const [addDecimals, setAddDecimals] = useState('0')

  const [roleSettings, setRoleSettings] = useState<RoleSettings | null>(null)
  const [roleDraft, setRoleDraft] = useState<RoleSettings | null>(null)
  const [rolesLoading, setRolesLoading] = useState(false)
  const [rolesError, setRolesError] = useState<string | null>(null)
  const [savingRoles, setSavingRoles] = useState(false)

  const toggleRoleDraft = (key: keyof RoleSettings) => {
    setRoleDraft((current) =>
      current
        ? {
          ...current,
          [key]: !current[key],
        }
        : current
    )
  }

  const handleSaveRoles = async () => {
    if (!roleDraft) return

    setSavingRoles(true)
    setRolesError(null)

    try {
      const tokenResponse = await fetch('/api/auth/token')
      if (!tokenResponse.ok) {
        throw new Error('You must be logged in to update roles')
      }

      const { accessToken } = await tokenResponse.json()
      if (!accessToken) {
        throw new Error('Missing access token')
      }

      const updated = await callMcpTool('update_role_settings', roleDraft, accessToken)
      setRoleSettings(updated)
      setRoleDraft(updated)
      setNotification({ type: 'success', message: 'Roles updated successfully' })
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to update roles'
      setRolesError(message)
    } finally {
      setSavingRoles(false)
    }
  }

  const [notification, setNotification] = useState<
    | {
        type: 'success' | 'error'
        message: string
      }
    | null
  >(null)

  useEffect(() => {
    if (!notification) return
    const timer = setTimeout(() => setNotification(null), 5000)
    return () => clearTimeout(timer)
  }, [notification])

  const loadAccounts = useCallback(async () => {
    setAccountsLoading(true)
    setAccountsError(false)
    setAccountsHasAccess(false)

    try {
      const tokenResponse = await fetch('/api/auth/token')
      if (!tokenResponse.ok) {
        setClientAccounts(null)
        setDeskAccounts(null)
        return
      }

      const { accessToken } = await tokenResponse.json()
      if (!accessToken) {
        setClientAccounts(null)
        setDeskAccounts(null)
        return
      }

      const data = await callMcpTool('list_accounts', {}, accessToken)

      const clients = data.client_accounts
        .filter((account) => account.account_type === 'Client')
        .map<ClientAccount>((account) => {
          const normalizedName =
            typeof account.name === 'string' && account.name.trim().length > 0
              ? account.name.trim()
              : null

          return {
            accountId: account.account_id,
            network: account.network === 'Localnet' ? 'Localnet' : 'Testnet',
            name: normalizedName,
          }
        })

      setClientAccounts(clients)
      setDeskOwnerAccount((current) => (current ? current : clients[0]?.accountId ?? ''))
      const desks = (data.desk_accounts as DeskAccountInfo[]).map<DeskAccount>((desk) => ({
        accountId: desk.account_id,
        network: desk.network === 'Localnet' ? 'Localnet' : 'Testnet',
        market: desk.market,
        ownerAccount: desk.owner_account,
        ownerName: clients.find((account) => account.accountId === desk.owner_account)?.name ?? null,
        marketUrl: desk.market_url,
      }))
      setDeskAccounts(desks)
      setAccountsHasAccess(true)
    } catch (err) {
      console.error('Error fetching client accounts:', err)
      setAccountsError(true)
      setClientAccounts(null)
      setDeskAccounts(null)
    } finally {
      setAccountsLoading(false)
    }
  }, [])

  const loadAssets = async () => {
    setLoading(true)
    setError(false)
    setHasAccess(false)

    try {
      let accessToken: string | null = null
      let fetchedToken = false

      try {
        const tokenResponse = await fetch('/api/auth/token')
        if (tokenResponse.ok) {
          const tokenPayload = await tokenResponse.json()
          accessToken = tokenPayload.accessToken ?? null
          fetchedToken = Boolean(accessToken)
        }
      } catch (tokenError) {
        console.warn('Unable to retrieve access token; using public asset list', tokenError)
      }

      const data = await callMcpTool('list_assets', {}, accessToken)
      setAssets(data)
      setHasAccess(fetchedToken)
    } catch (err) {
      console.error('Error fetching assets:', err)
      setError(true)
      setAssets(null)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadAccounts()
  }, [loadAccounts])

  useEffect(() => {
    void loadAssets()
  }, [])

  useEffect(() => {
    if (!createDeskModalOpen) {
      setDeskBaseAccount('')
      setDeskQuoteAccount('')
      setDeskError(null)
      setCreatingDesk(false)
    }
  }, [createDeskModalOpen])

  const loadRoleSettings = useCallback(async () => {
    setRolesLoading(true)
    setRolesError(null)

    try {
      const tokenResponse = await fetch('/api/auth/token')
      if (!tokenResponse.ok) {
        setRoleSettings(null)
        setRoleDraft(null)
        return
      }

      const { accessToken } = await tokenResponse.json()
      if (!accessToken) {
        setRoleSettings(null)
        setRoleDraft(null)
        return
      }

      const settings = await callMcpTool('get_role_settings', {}, accessToken)
      setRoleSettings(settings)
      setRoleDraft(settings)
    } catch (err) {
      console.error('Failed to load role settings', err)
      setRolesError('Unable to load role settings')
      setRoleSettings(null)
      setRoleDraft(null)
    } finally {
      setRolesLoading(false)
    }
  }, [])

  useEffect(() => {
    void loadRoleSettings()
  }, [loadRoleSettings])

  const rolesChanged = useMemo(() => {
    if (!roleSettings || !roleDraft) return false
    return (
      roleSettings.is_client !== roleDraft.is_client ||
      roleSettings.is_liquidity_provider !== roleDraft.is_liquidity_provider ||
      roleSettings.is_desk !== roleDraft.is_desk
    )
  }, [roleSettings, roleDraft])

  const handleCreateClientAccount = async () => {
    const trimmedName = accountName.trim()
    if (!trimmedName) {
      setNotification({
        type: 'error',
        message: 'Account name is required',
      })
      return
    }

    setCreatingAccount(true)

    try {
      const tokenResponse = await fetch('/api/auth/token')
      if (!tokenResponse.ok) {
        throw new Error('You must be logged in to create accounts')
      }

      const { accessToken } = await tokenResponse.json()
      if (!accessToken) {
        throw new Error('Missing access token')
      }

      await callMcpTool(
        'create_account_order',
        {
          order: {
            CreateClient: {
              network: accountNetwork,
              name: trimmedName,
            },
          },
        },
        accessToken
      )

      await loadAccounts()

      setCreateAccountModalOpen(false)
      setAccountName('')
      setAccountNetwork('Testnet')

      setNotification({
        type: 'success',
        message: `${trimmedName} account created successfully`,
      })
    } catch (err) {
      console.error('Error creating client account:', err)
      setNotification({
        type: 'error',
        message: err instanceof Error ? err.message : 'Unknown error',
      })
    } finally {
      setCreatingAccount(false)
    }
  }

  const inferNetworkFromAccount = (accountId: string): NetworkName =>
    accountId.toLowerCase().startsWith('mtst') ? 'Testnet' : 'Localnet'

  const handleCreateDesk = async () => {
    setDeskError(null)

    if (!roleSettings?.is_desk) {
      setDeskError('You must be a desk manager to create a desk.')
      return
    }

    if (!clientAccounts || clientAccounts.length === 0) {
      setDeskError('You must have at least one client account before creating a desk.')
      return
    }

    if (!assets || assets.length < 2) {
      setDeskError('You need at least two assets before creating a desk.')
      return
    }

    if (!deskOwnerAccount) {
      setDeskError('Select a client account to own this desk.')
      return
    }

    const baseAsset = assets.find((asset) => asset.account === deskBaseAccount)
    const quoteAsset = assets.find((asset) => asset.account === deskQuoteAccount)

    if (!baseAsset) {
      setDeskError('Selected base asset could not be found.')
      return
    }

    if (!quoteAsset) {
      setDeskError('Selected quote asset could not be found.')
      return
    }

    if (baseAsset.account === quoteAsset.account) {
      setDeskError('Base and quote assets must be different.')
      return
    }

    const baseNetwork = inferNetworkFromAccount(baseAsset.account)
    const quoteNetwork = inferNetworkFromAccount(quoteAsset.account)

    if (baseNetwork !== quoteNetwork) {
      setDeskError('Base and quote assets must be on the same network.')
      return
    }

    const ownerAccount = clientAccounts.find((account) => account.accountId === deskOwnerAccount)
    if (!ownerAccount) {
      setDeskError('The selected owner account is no longer available.')
      return
    }

    if (ownerAccount.network !== baseNetwork) {
      setDeskError('Owner account must be on the same network as the selected assets.')
      return
    }

    if (
      ownerAccount.accountId === baseAsset.account ||
      ownerAccount.accountId === quoteAsset.account
    ) {
      setDeskError('Owner account must be different from the asset accounts.')
      return
    }

    setCreatingDesk(true)

    try {
      const tokenResponse = await fetch('/api/auth/token')
      if (!tokenResponse.ok) {
        throw new Error('You must be logged in to create a desk')
      }

      const { accessToken } = await tokenResponse.json()
      if (!accessToken) {
        throw new Error('Missing access token')
      }

      const createDeskResponse = (await callMcpTool(
        'create_account_order',
        {
          order: {
            CreateDesk: {
              network: baseNetwork,
              market: {
                base: { code: baseAsset.symbol, issuer: baseAsset.account },
                quote: { code: quoteAsset.symbol, issuer: quoteAsset.account },
              },
              owner_account: ownerAccount.accountId,
            },
          },
        },
        accessToken,
      )) as CreateAccountOrderResponse

      if (
        createDeskResponse.success &&
        'Desk' in createDeskResponse.result &&
        createDeskResponse.result.Desk
      ) {
        const deskResult = createDeskResponse.result.Desk
        marketStorage.saveMarket({
          pair: `${deskResult.market.base.code}/${deskResult.market.quote.code}`,
          marketId: deskResult.account_id,
          baseFaucet: deskResult.market.base.issuer,
          quoteFaucet: deskResult.market.quote.issuer,
          deskUrl: deskResult.market_url,
        })
      }

      setNotification({ type: 'success', message: 'Desk created successfully' })
      setCreateDeskModalOpen(false)
      await loadAccounts()
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to create desk'
      setDeskError(message)
    } finally {
      setCreatingDesk(false)
    }
  }

  const handleCreateAsset = async () => {
    setCreating(true)

    try {
      const tokenResponse = await fetch('/api/auth/token')
      if (!tokenResponse.ok) {
        throw new Error('Failed to get access token')
      }

      const { accessToken } = await tokenResponse.json()
      if (!accessToken) {
        throw new Error('Missing access token')
      }

      const decimalsValue = parseInt(decimals, 10)
      if (Number.isNaN(decimalsValue) || decimalsValue < 0 || decimalsValue > 18) {
        throw new Error('Decimals must be between 0 and 18')
      }

      const normalizedSupply = maxSupply.replace(/[,_\s]/g, '')
      if (!/^\d+$/.test(normalizedSupply)) {
        throw new Error('Max supply must contain digits only (commas are allowed)')
      }

      const baseSupply = BigInt(normalizedSupply)
      const scale = BigInt(10) ** BigInt(decimalsValue)
      const scaledSupply = baseSupply * scale

      if (scaledSupply > BigInt(Number.MAX_SAFE_INTEGER)) {
        throw new Error('Max supply is too large to represent safely. Reduce the value or decimals.')
      }

      await callMcpTool(
        'create_account_order',
        {
          order: {
            CreateFaucet: {
              token_symbol: tokenSymbol,
              decimals: decimalsValue,
              max_supply: Number(scaledSupply),
              network: network as NetworkName,
            },
          },
        },
        accessToken
      )

      setCreateModalOpen(false)
      setTokenSymbol('')
      setDecimals('8')
      setMaxSupply('')
      setNetwork('Testnet')

      await loadAssets()

      setNotification({
        type: 'success',
        message: `${tokenSymbol} faucet registered successfully`,
      })
    } catch (err) {
      console.error('Error creating faucet:', err)
      setNotification({
        type: 'error',
        message: err instanceof Error ? err.message : 'Unknown error',
      })
    } finally {
      setCreating(false)
    }
  }

  const handleAddAsset = async () => {
    setAdding(true)

    try {
      const tokenResponse = await fetch('/api/auth/token')
      if (!tokenResponse.ok) {
        throw new Error('You must be logged in to link assets')
      }

      const { accessToken } = await tokenResponse.json()
      if (!accessToken) {
        throw new Error('Missing access token')
      }

      const symbol = addSymbol.trim().toUpperCase()
      const account = addAccount.trim()

      await callMcpTool(
        'register_asset',
        {
          symbol,
          account,
          max_supply: '0',
          decimals: parseInt(addDecimals || '0', 10),
          verified: false,
          owner: false,
          hidden: false,
        },
        accessToken
      )

      setAddModalOpen(false)
      setAddSymbol('')
      setAddAccount('')
      setAddDecimals('0')

      await loadAssets()

      setNotification({
        type: 'success',
        message: `${symbol || 'Asset'} linked successfully`,
      })
    } catch (err) {
      console.error('Error linking asset:', err)
      setNotification({
        type: 'error',
        message: err instanceof Error ? err.message : 'Unknown error',
      })
    } finally {
      setAdding(false)
    }
  }

  return (
    <div className="space-y-10">
      {notification && (
        <div
          className={clsx(
            'fixed right-6 top-6 z-50 w-full max-w-sm rounded-md p-4 shadow-lg',
            notification.type === 'success'
              ? 'bg-emerald-50 border border-emerald-200 text-emerald-800'
              : 'bg-red-50 border border-red-200 text-red-800'
          )}
        >
          <p className="text-sm font-medium">{notification.message}</p>
        </div>
      )}

      <section>
        <Card className="p-6 bg-card border-border">
          <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
            <div>
              <h2 className="text-2xl font-semibold text-foreground">Role Settings</h2>
              <p className="text-sm text-muted-foreground">
                Choose the roles you want to enable for this workspace.
              </p>
            </div>
            <Button
              onClick={handleSaveRoles}
              disabled={
                !roleDraft || rolesLoading || savingRoles || !rolesChanged
              }
            >
              {savingRoles && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Save Roles
            </Button>
          </div>

          {rolesLoading && (
            <p className="mt-4 text-sm text-muted-foreground flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading role settings...
            </p>
          )}

          {rolesError && (
            <p className="mt-4 text-sm text-destructive">{rolesError}</p>
          )}

          {!rolesLoading && !roleDraft && !rolesError && (
            <p className="mt-4 text-sm text-muted-foreground">
              Sign in to configure your role preferences.
            </p>
          )}

          {roleDraft && (
            <div className="mt-6 grid gap-4 md:grid-cols-3">
              <label className="flex items-start gap-3 rounded-lg border border-border/60 bg-muted/30 p-4 hover:border-primary transition-colors">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4"
                  checked={roleDraft.is_client}
                  onChange={() => toggleRoleDraft('is_client')}
                  disabled={rolesLoading || savingRoles}
                />
                <div>
                  <p className="font-semibold text-foreground">Client</p>
                  <p className="text-sm text-muted-foreground">
                    Access OTC markets and manage client accounts.
                  </p>
                </div>
              </label>

              <label className="flex items-start gap-3 rounded-lg border border-border/60 bg-muted/30 p-4 hover:border-primary transition-colors">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4"
                  checked={roleDraft.is_liquidity_provider}
                  onChange={() => toggleRoleDraft('is_liquidity_provider')}
                  disabled={rolesLoading || savingRoles}
                />
                <div>
                  <p className="font-semibold text-foreground">Liquidity Provider</p>
                  <p className="text-sm text-muted-foreground">
                    Provide quotes and manage liquidity offers for markets.
                  </p>
                </div>
              </label>

              <label className="flex items-start gap-3 rounded-lg border border-border/60 bg-muted/30 p-4 hover:border-primary transition-colors">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4"
                  checked={roleDraft.is_desk}
                  onChange={() => toggleRoleDraft('is_desk')}
                  disabled={rolesLoading || savingRoles}
                />
                <div>
                  <p className="font-semibold text-foreground">Desk Manager</p>
                  <p className="text-sm text-muted-foreground">
                    Manage markets, desks, and oversee trading activity.
                  </p>
                </div>
              </label>
            </div>
          )}
        </Card>
      </section>

      <section>
        <div className="mb-6 flex items-center justify-between">
          <h2 className="text-2xl font-semibold">Accounts</h2>
          <Dialog open={createAccountModalOpen} onOpenChange={setCreateAccountModalOpen}>
            <DialogTrigger asChild>
              <Button variant="outline" disabled={!accountsHasAccess}>
                <Plus className="mr-2 h-4 w-4" />
                Create Account
              </Button>
            </DialogTrigger>
            <DialogContent className="sm:max-w-[500px]">
              <DialogHeader>
                <DialogTitle>Create Client Account</DialogTitle>
                <DialogDescription>
                  Create a new client account to interact with Mosaic services.
                </DialogDescription>
              </DialogHeader>
              <div className="grid gap-4 py-4">
                <div className="grid gap-2">
                  <Label htmlFor="accountName">Name</Label>
                  <Input
                    id="accountName"
                    placeholder="e.g., Primary Client"
                    value={accountName}
                    onChange={(e) => setAccountName(e.target.value)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="accountNetwork">Network</Label>
                  <Select value={accountNetwork} onValueChange={(value) => setAccountNetwork(value as NetworkName)}>
                    <SelectTrigger id="accountNetwork">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Testnet">Testnet</SelectItem>
                      <SelectItem value="Localnet">Localnet</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setCreateAccountModalOpen(false)} disabled={creatingAccount}>
                  Cancel
                </Button>
                <Button onClick={handleCreateClientAccount} disabled={creatingAccount || !accountName.trim()}>
                  {creatingAccount && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                  {creatingAccount ? 'Creating...' : 'Create Account'}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </div>

        {accountsLoading && (
          <div className="grid gap-4" style={{ fontFamily: 'var(--font-dm-mono)' }}>
            {[1, 2].map((i) => (
              <Card key={i} className="p-6 bg-card border-border animate-pulse">
                <div className="flex items-center gap-4">
                  <div className="h-12 w-12 rounded-full bg-muted" />
                  <div className="flex-1 space-y-2">
                    <div className="h-6 w-32 bg-muted rounded" />
                    <div className="h-4 w-48 bg-muted rounded" />
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}

        {!accountsLoading && accountsError && (
          <Card className="p-8 bg-card border-border">
            <div className="flex flex-col items-center justify-center gap-4 text-center">
              <div className="h-16 w-16 rounded-full bg-destructive/10 flex items-center justify-center">
                <AlertCircle className="h-8 w-8 text-destructive" />
              </div>
              <div>
                <h3 className="text-xl font-semibold text-foreground mb-2">Unable to Load Accounts</h3>
                <p className="text-muted-foreground max-w-md mb-4">
                  We couldn&apos;t retrieve your client accounts at this time. Please try again.
                </p>
                <Button onClick={() => { void loadAccounts() }}>Retry</Button>
              </div>
            </div>
          </Card>
        )}

        {!accountsLoading && !accountsError && !accountsHasAccess && (
          <Card className="p-8 bg-card border-border">
            <div className="text-center space-y-2">
              <h3 className="text-xl font-semibold text-foreground">Log in to manage accounts</h3>
              <p className="text-muted-foreground">
                Sign in to view your Mosaic client accounts and create new ones.
              </p>
            </div>
          </Card>
        )}

        {!accountsLoading && !accountsError && accountsHasAccess && clientAccounts && clientAccounts.length === 0 && (
          <Card className="p-8 bg-card border-border">
            <div className="text-center space-y-2">
              <h3 className="text-xl font-semibold text-foreground">No client accounts yet</h3>
              <p className="text-muted-foreground">
                Create your first client account to start interacting with Mosaic.
              </p>
            </div>
          </Card>
        )}

        {!accountsLoading && !accountsError && accountsHasAccess && clientAccounts && clientAccounts.length > 0 && (
          <div className="grid gap-4" style={{ fontFamily: 'var(--font-dm-mono)' }}>
            {clientAccounts.map((account) => (
              <Card key={account.accountId} className="p-6 bg-card border-border">
                <div className="flex items-center gap-4">
                  <div className="h-12 w-12 rounded-full bg-primary/10 flex items-center justify-center">
                    <Wallet className="h-6 w-6 text-primary" />
                  </div>
                  <div>
                    <div className="flex items-center gap-3 mb-1">
                      <h3 className="text-xl font-semibold text-foreground">
                        {account.name ?? 'Client Account'}
                      </h3>
                      <Badge variant="outline" className="text-xs">
                        {account.network}
                      </Badge>
                      <Badge variant="outline" className="text-xs">
                        Client
                      </Badge>
                      <Badge variant="outline" className="text-xs">
                        Private
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Account:{' '}
                      {account.accountId.startsWith('mtst') ? (
                        <a
                          href={`https://testnet.midenscan.com/account/${account.accountId}`}
                          className="text-primary underline-offset-2 hover:underline"
                          rel="noreferrer"
                          target="_blank"
                        >
                          {account.accountId}
                        </a>
                      ) : (
                        account.accountId
                      )}
                    </p>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
      </section>

      <section className="mt-12">
        <div className="mb-6 flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <h2 className="text-2xl font-semibold">Desks</h2>
            <p className="text-sm text-muted-foreground">
              Overview of desk accounts tied to your organization and their market configuration.
            </p>
          </div>
          <Dialog
            open={createDeskModalOpen}
            onOpenChange={(open) => {
              setCreateDeskModalOpen(open)
              if (!open) {
                setDeskBaseAccount('')
                setDeskQuoteAccount('')
                setDeskOwnerAccount((clientAccounts && clientAccounts[0]?.accountId) || '')
                setDeskError(null)
              }
            }}
          >
            <DialogTrigger asChild>
              <Button
                variant="outline"
                disabled={
                  !roleSettings?.is_desk ||
                  !assets ||
                  assets.length < 2 ||
                  creatingDesk ||
                  !clientAccounts ||
                  clientAccounts.length === 0
                }
              >
                <Plus className="mr-2 h-4 w-4" />
                Create Desk
              </Button>
            </DialogTrigger>
            <DialogContent className="sm:max-w-[500px]">
              <DialogHeader>
                <DialogTitle>Create Desk</DialogTitle>
                <DialogDescription>
                  Choose base and quote assets to initialize a new desk account. Assets must exist in
                  your catalog and reside on the same network.
                </DialogDescription>
              </DialogHeader>
              <div className="grid gap-4 py-4">
                <div className="grid gap-2">
                  <Label htmlFor="deskOwnerAccount">Desk Owner</Label>
                  <Select
                    value={deskOwnerAccount}
                    onValueChange={(value) => {
                      setDeskOwnerAccount(value)
                      setDeskError(null)
                    }}
                    disabled={!clientAccounts || clientAccounts.length === 0}
                  >
                    <SelectTrigger id="deskOwnerAccount">
                      <SelectValue placeholder="Select owner account" />
                    </SelectTrigger>
                    <SelectContent>
                      {clientAccounts && clientAccounts.length > 0 ? (
                        clientAccounts.map((account) => (
                          <SelectItem key={account.accountId} value={account.accountId}>
                            {account.name
                              ? `${account.name} · ${account.accountId}`
                              : account.accountId}
                          </SelectItem>
                        ))
                      ) : (
                        <SelectItem value="" disabled>
                          No client accounts available
                        </SelectItem>
                      )}
                    </SelectContent>
                  </Select>
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="deskBaseAsset">Base Asset</Label>
                  <Select
                    value={deskBaseAccount}
                    onValueChange={(value) => {
                      setDeskBaseAccount(value)
                      setDeskError(null)
                    }}
                    disabled={!assets || assets.length === 0}
                  >
                    <SelectTrigger id="deskBaseAsset">
                      <SelectValue placeholder="Select base asset" />
                    </SelectTrigger>
                    <SelectContent>
                      {(assets ?? []).map((asset) => (
                        <SelectItem key={asset.account} value={asset.account}>
                          {asset.symbol} · {asset.account}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="deskQuoteAsset">Quote Asset</Label>
                  <Select
                    value={deskQuoteAccount}
                    onValueChange={(value) => {
                      setDeskQuoteAccount(value)
                      setDeskError(null)
                    }}
                    disabled={!assets || assets.length === 0}
                  >
                    <SelectTrigger id="deskQuoteAsset">
                      <SelectValue placeholder="Select quote asset" />
                    </SelectTrigger>
                    <SelectContent>
                      {(assets ?? []).map((asset) => (
                        <SelectItem key={asset.account} value={asset.account}>
                          {asset.symbol} · {asset.account}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <p className="text-sm text-muted-foreground">
                  Both assets must already exist and belong to the same network. Base and quote assets
                  must be different.
                </p>
                {deskError && <p className="text-sm text-destructive">{deskError}</p>}
              </div>
              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={() => setCreateDeskModalOpen(false)}
                  disabled={creatingDesk}
                >
                  Cancel
                </Button>
                <Button
                  onClick={() => {
                    void handleCreateDesk()
                  }}
                  disabled={
                    creatingDesk ||
                    !deskOwnerAccount ||
                    !deskBaseAccount ||
                    !deskQuoteAccount ||
                    deskBaseAccount === deskQuoteAccount
                  }
                >
                  {creatingDesk && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                  {creatingDesk ? 'Creating...' : 'Create Desk'}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </div>

        {accountsLoading && (
          <div className="grid gap-4" style={{ fontFamily: 'var(--font-dm-mono)' }}>
            {[1, 2].map((i) => (
              <Card key={i} className="p-6 bg-card border-border animate-pulse">
                <div className="flex items-center gap-4">
                  <div className="h-12 w-12 rounded-full bg-muted" />
                  <div className="flex-1 space-y-2">
                    <div className="h-6 w-36 bg-muted rounded" />
                    <div className="h-4 w-52 bg-muted rounded" />
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}

        {!accountsLoading && accountsError && (
          <Card className="p-8 bg-card border-border">
            <div className="flex flex-col items-center justify-center gap-4 text-center">
              <div className="h-16 w-16 rounded-full bg-destructive/10 flex items-center justify-center">
                <AlertCircle className="h-8 w-8 text-destructive" />
              </div>
              <div>
                <h3 className="text-xl font-semibold text-foreground mb-2">Unable to Load Desks</h3>
                <p className="text-muted-foreground max-w-md mb-4">
                  We weren&apos;t able to retrieve your desk accounts. Please try again.
                </p>
                <Button onClick={() => { void loadAccounts() }}>Retry</Button>
              </div>
            </div>
          </Card>
        )}

        {!accountsLoading && !accountsError && !accountsHasAccess && (
          <Card className="p-8 bg-card border-border">
            <div className="text-center space-y-2">
              <h3 className="text-xl font-semibold text-foreground">Log in to view desks</h3>
              <p className="text-muted-foreground">Sign in to review desk accounts and their allocated markets.</p>
            </div>
          </Card>
        )}

        {!accountsLoading && !accountsError && accountsHasAccess && deskAccounts && deskAccounts.length === 0 && (
          <Card className="p-8 bg-card border-border">
            <div className="text-center space-y-2">
              <h3 className="text-xl font-semibold text-foreground">No desk accounts</h3>
              <p className="text-muted-foreground">Promote a client to desk status to manage OTC liquidity.</p>
            </div>
          </Card>
        )}

        {!accountsLoading && !accountsError && accountsHasAccess && deskAccounts && deskAccounts.length > 0 && (
          <div className="grid gap-4" style={{ fontFamily: 'var(--font-dm-mono)' }}>
            {deskAccounts.map((desk) => {
              const marketLabel = `${desk.market.base.code}/${desk.market.quote.code}`
              const linkClassName =
                'text-xl font-semibold text-primary underline-offset-2 hover:underline'

              return (
                <Card key={desk.accountId} className="p-6 bg-card border-border">
                  <div className="flex items-center gap-4">
                    <div className="h-12 w-12 rounded-full bg-primary/10 flex items-center justify-center">
                      <Briefcase className="h-6 w-6 text-primary" />
                    </div>
                    <div className="flex-1">
                      <div className="flex flex-wrap items-center gap-2 mb-2">
                        <Link href={`/desk/${desk.accountId}`} className={linkClassName}>
                          {marketLabel}
                        </Link>
                        <Badge variant="outline" className="text-xs">
                          {desk.network}
                        </Badge>
                      </div>
                      <div className="space-y-1 text-sm text-muted-foreground">
                        <div>
                          Desk Account:{' '}
                          {desk.accountId.startsWith('mtst') ? (
                            <a
                              href={`https://testnet.midenscan.com/account/${desk.accountId}`}
                              className="text-primary underline-offset-2 hover:underline"
                              rel="noreferrer"
                              target="_blank"
                            >
                              {desk.accountId}
                            </a>
                          ) : (
                            <span className="font-mono break-all">{desk.accountId}</span>
                          )}
                        </div>
                        <div>
                          Base:{' '}
                          <span className="font-semibold text-foreground">
                            {desk.market.base.code}
                          </span>{' '}
                          (
                          {desk.market.base.issuer.startsWith('mtst') ? (
                            <a
                              href={`https://testnet.midenscan.com/account/${desk.market.base.issuer}`}
                              className="text-primary underline-offset-2 hover:underline"
                              rel="noreferrer"
                              target="_blank"
                            >
                              {desk.market.base.issuer}
                            </a>
                          ) : (
                            <span className="font-mono break-all">
                              {desk.market.base.issuer}
                            </span>
                          )}
                          )
                        </div>
                        <div>
                          Quote:{' '}
                          <span className="font-semibold text-foreground">
                            {desk.market.quote.code}
                          </span>{' '}
                          (
                          {desk.market.quote.issuer.startsWith('mtst') ? (
                            <a
                              href={`https://testnet.midenscan.com/account/${desk.market.quote.issuer}`}
                              className="text-primary underline-offset-2 hover:underline"
                              rel="noreferrer"
                              target="_blank"
                            >
                              {desk.market.quote.issuer}
                            </a>
                          ) : (
                            <span className="font-mono break-all">
                              {desk.market.quote.issuer}
                            </span>
                          )}
                          )
                        </div>
                        <div className="flex items-center gap-2">
                          <span>Routing URL:{' '}
                            <span className="text-primary underline-offset-2">
                              {desk.marketUrl}
                            </span>
                          </span>
                          <button
                            type="button"
                            className="inline-flex h-6 w-6 items-center justify-center rounded-full border border-border bg-muted text-muted-foreground hover:bg-primary/10 hover:text-primary transition-colors"
                            onClick={async () => {
                              try {
                                await navigator.clipboard.writeText(desk.marketUrl)
                              } catch (err) {
                                console.warn('Failed to copy Routing URL:', err)
                              }
                            }}
                            title="Copy Routing URL"
                          >
                            <Copy className="h-3 w-3" />
                          </button>
                        </div>
                        <div>
                          Owner:{' '}
                          <span className="font-semibold text-foreground">
                            {desk.ownerName ?? 'Client Account'}
                          </span>{' '}
                          (
                          {desk.ownerAccount.startsWith('mtst') ? (
                            <a
                              href={`https://testnet.midenscan.com/account/${desk.ownerAccount}`}
                              className="text-primary underline-offset-2 hover:underline"
                              rel="noreferrer"
                              target="_blank"
                            >
                              {desk.ownerAccount}
                            </a>
                          ) : (
                            <span className="font-mono break-all">{desk.ownerAccount}</span>
                          )}
                          )
                        </div>
                      </div>
                    </div>
                  </div>
                </Card>
              )
            })}
          </div>
        )}
      </section>

      <section>
        <div className="mb-6 flex items-center justify-between">
          <h2 className="text-2xl font-semibold">Assets</h2>
          <div className="flex gap-2">
            <Dialog open={createModalOpen} onOpenChange={setCreateModalOpen}>
              <DialogTrigger asChild>
                <Button variant="outline" disabled={!hasAccess}>
                  <Plus className="mr-2 h-4 w-4" />
                  Create Faucet
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[500px]">
                <DialogHeader>
                  <DialogTitle>Create New Faucet Asset</DialogTitle>
                  <DialogDescription>
                    Create a new faucet account for a token. This will generate a new asset that can be used in the Mosaic ecosystem.
                  </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                  <div className="grid gap-2">
                    <Label htmlFor="symbol">Token Symbol</Label>
                    <Input
                      id="symbol"
                      placeholder="e.g., BTC, ETH, MID"
                      value={tokenSymbol}
                      onChange={(e) => setTokenSymbol(e.target.value.toUpperCase())}
                      maxLength={10}
                    />
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor="decimals">Decimals</Label>
                    <Input
                      id="decimals"
                      type="number"
                      min="0"
                      max="18"
                      value={decimals}
                      onChange={(e) => setDecimals(e.target.value)}
                    />
                    <p className="text-sm text-muted-foreground">
                      Number of decimal places (0-18). Common values: 8 for BTC, 18 for ETH
                    </p>
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor="maxSupply">Max Supply</Label>
                    <Input
                      id="maxSupply"
                      type="text"
                      inputMode="numeric"
                      placeholder="e.g., 1,000,000,000"
                      value={maxSupply}
                      onChange={(e) => setMaxSupply(e.target.value.replace(/[^\d,]/g, ''))}
                    />
                    <p className="text-sm text-muted-foreground">
                      Enter the total token supply in whole units (commas are optional). The value will be
                      converted to base units using the decimals provided.
                    </p>
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor="network">Network</Label>
                    <Select value={network} onValueChange={setNetwork}>
                      <SelectTrigger id="network">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="Testnet">Testnet</SelectItem>
                        <SelectItem value="Localnet">Localnet</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setCreateModalOpen(false)} disabled={creating}>
                    Cancel
                  </Button>
                  <Button onClick={handleCreateAsset} disabled={creating || !tokenSymbol || !maxSupply}>
                    {creating && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                    {creating ? 'Creating...' : 'Create Faucet'}
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>

            <Dialog open={addModalOpen} onOpenChange={setAddModalOpen}>
              <DialogTrigger asChild>
                <Button variant="outline" disabled={!hasAccess}>
                  <Plus className="mr-2 h-4 w-4" />
                  Link Asset
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[500px]">
                <DialogHeader>
                  <DialogTitle>Link Existing Asset</DialogTitle>
                  <DialogDescription>
                    Link an existing asset by providing its account ID, symbol, and decimals. Max supply is stored as unknown.
                  </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                  <div className="grid gap-2">
                    <Label htmlFor="addSymbol">Token Symbol</Label>
                    <Input
                      id="addSymbol"
                      placeholder="e.g., USDC"
                      value={addSymbol}
                      onChange={(e) => setAddSymbol(e.target.value.toUpperCase())}
                      maxLength={10}
                    />
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor="addAccount">Account ID (bech32)</Label>
                    <Input
                      id="addAccount"
                      placeholder="mtst1..."
                      value={addAccount}
                      onChange={(e) => setAddAccount(e.target.value)}
                    />
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor="addDecimals">Decimals</Label>
                    <Input
                      id="addDecimals"
                      type="number"
                      min="0"
                      max="18"
                      value={addDecimals}
                      onChange={(e) => setAddDecimals(e.target.value)}
                    />
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setAddModalOpen(false)} disabled={adding}>
                    Cancel
                  </Button>
                  <Button onClick={handleAddAsset} disabled={adding || !addSymbol || !addAccount}>
                    {adding && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                    {adding ? 'Linking...' : 'Link Asset'}
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        </div>

        {loading && (
          <div className="grid gap-4" style={{ fontFamily: 'var(--font-dm-mono)' }}>
            {[1, 2, 3].map((i) => (
              <Card key={i} className="p-6 bg-card border-border animate-pulse">
                <div className="flex items-center gap-4">
                  <div className="h-12 w-12 rounded-full bg-muted" />
                  <div className="flex-1 space-y-2">
                    <div className="h-6 w-24 bg-muted rounded" />
                    <div className="h-4 w-64 bg-muted rounded" />
                    <div className="h-4 w-48 bg-muted rounded" />
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}

        {!loading && error && (
          <Card className="p-8 bg-card border-border">
            <div className="flex flex-col items-center justify-center gap-4 text-center">
              <div className="h-16 w-16 rounded-full bg-destructive/10 flex items-center justify-center">
                <AlertCircle className="h-8 w-8 text-destructive" />
              </div>
              <div>
                <h3 className="text-xl font-semibold text-foreground mb-2">Unable to Load Assets</h3>
                <p className="text-muted-foreground max-w-md mb-4">
                  We couldn&apos;t retrieve the asset list at this time. The server may be unavailable or the request timed out.
                </p>
                <Button onClick={loadAssets}>Retry</Button>
              </div>
            </div>
          </Card>
        )}

        {!loading && !error && assets && (
          <div className="grid gap-4" style={{ fontFamily: 'var(--font-dm-mono)' }}>
            {assets.map((asset) => (
              <Card
                key={asset.account}
                className={clsx(
                  'p-6 bg-card border-border transition-colors',
                  asset.verified ? 'hover:border-primary/50' : 'border-red-500/50 hover:border-red-500'
                )}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className="h-12 w-12 rounded-full bg-primary/10 flex items-center justify-center">
                      <Coins className="h-6 w-6 text-primary" />
                    </div>
                    <div>
                      <div className="flex items-center gap-3 mb-1">
                        <h3 className="text-xl font-semibold text-foreground">{asset.symbol}</h3>
                        <Badge variant={asset.verified ? 'outline' : 'destructive'} className="text-xs">
                          {asset.verified ? 'Verified' : 'Unverified'}
                        </Badge>
                        {asset.owner && (
                          <Badge variant="outline" className="text-xs">
                            Owner
                          </Badge>
                        )}
                        {asset.account.startsWith('mtst') && (
                          <Badge variant="outline" className="text-xs">
                            Testnet
                          </Badge>
                        )}
                        <Badge variant="outline" className="text-xs">
                          Public
                        </Badge>
                      </div>
                      <p className="text-sm text-muted-foreground">
                        Account:{' '}
                        {asset.account.startsWith('mtst') ? (
                          <a
                            href={`https://testnet.midenscan.com/account/${asset.account}`}
                            className="text-primary underline-offset-2 hover:underline"
                            rel="noreferrer"
                            target="_blank"
                          >
                            {asset.account}
                          </a>
                        ) : (
                          asset.account
                        )}
                      </p>
                      <p className="text-sm text-muted-foreground">
                        Max Supply: {formatAssetSupply(asset.maxSupply, asset.decimals)} (decimals: {asset.decimals})
                      </p>
                    </div>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
      </section>
    </div>
  )
}
