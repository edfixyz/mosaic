'use client'

import Link from 'next/link'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { ArrowRight, TrendingUp, Shield, Zap, Loader2 } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { RoleSettings, callMcpTool } from '@/lib/mcp-tool'

const defaultRoles: RoleSettings = {
  is_client: false,
  is_liquidity_provider: false,
  is_desk: false,
}

export default function HomePage() {
  const [roleSettings, setRoleSettings] = useState<RoleSettings | null>(null)
  const [roleDraft, setRoleDraft] = useState<RoleSettings>(defaultRoles)
  const [rolesLoading, setRolesLoading] = useState(false)
  const [rolesError, setRolesError] = useState<string | null>(null)
  const [savingRoles, setSavingRoles] = useState(false)
  const [roleModalOpen, setRoleModalOpen] = useState(false)
  const [promptDismissed, setPromptDismissed] = useState(false)

  const loadRoleSettings = useCallback(async () => {
    setRolesLoading(true)
    setRolesError(null)
    try {
      const tokenResponse = await fetch('/api/auth/token')
      if (!tokenResponse.ok) {
        setRoleSettings(null)
        return
      }

      const { accessToken } = await tokenResponse.json()
      if (!accessToken) {
        setRoleSettings(null)
        return
      }

      const settings = await callMcpTool('get_role_settings', {}, accessToken)
      setRoleSettings(settings)
      setRoleDraft(settings)
      setPromptDismissed(false)
    } catch (err) {
      console.error('Failed to load role settings', err)
      setRolesError('Unable to load role settings')
      setRoleSettings(null)
      setRoleDraft(defaultRoles)
    } finally {
      setRolesLoading(false)
    }
  }, [])

  useEffect(() => {
    void loadRoleSettings()
  }, [loadRoleSettings])

  const rolesChanged = useMemo(() => {
    if (!roleSettings) {
      return (
        roleDraft.is_client ||
        roleDraft.is_liquidity_provider ||
        roleDraft.is_desk
      )
    }

    return (
      roleSettings.is_client !== roleDraft.is_client ||
      roleSettings.is_liquidity_provider !== roleDraft.is_liquidity_provider ||
      roleSettings.is_desk !== roleDraft.is_desk
    )
  }, [roleSettings, roleDraft])

  const shouldPrompt = useMemo(() => {
    if (!roleSettings) return false
    return (
      !roleSettings.is_client &&
      !roleSettings.is_liquidity_provider &&
      !roleSettings.is_desk
    )
  }, [roleSettings])

  useEffect(() => {
    if (shouldPrompt && !promptDismissed) {
      setRoleModalOpen(true)
    }
  }, [shouldPrompt, promptDismissed])

  const toggleRole = (key: keyof RoleSettings) => {
    setRoleDraft((current) => ({
      ...current,
      [key]: !current[key],
    }))
  }

  const handleRoleModalChange = (open: boolean) => {
    setRoleModalOpen(open)
    if (!open) {
      setPromptDismissed(true)
    }
  }

  const handleSaveRoles = async () => {
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
      setRoleModalOpen(false)
      setPromptDismissed(false)
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to update roles'
      setRolesError(message)
    } finally {
      setSavingRoles(false)
    }
  }

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
            <Button
              size="lg"
              variant="outline"
              onClick={() => {
                setPromptDismissed(false)
                setRoleModalOpen(true)
              }}
              disabled={rolesLoading}
            >
              Manage Roles
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

      <Dialog open={roleModalOpen} onOpenChange={handleRoleModalChange}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>Select Your Roles</DialogTitle>
            <DialogDescription>
              Choose the roles you want to enable. You can update these at any time from the workbench.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            {rolesError && <p className="text-sm text-destructive">{rolesError}</p>}
            <div className="space-y-3">
              <label className="flex items-start gap-3 rounded-lg border border-border/60 bg-muted/30 p-4 hover:border-primary transition-colors">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4"
                  checked={roleDraft.is_client}
                  onChange={() => toggleRole('is_client')}
                  disabled={savingRoles}
                />
                <div>
                  <p className="font-semibold text-foreground">Client</p>
                  <p className="text-sm text-muted-foreground">
                    Trade on Mosaic markets and manage client accounts.
                  </p>
                </div>
              </label>

              <label className="flex items-start gap-3 rounded-lg border border-border/60 bg-muted/30 p-4 hover:border-primary transition-colors">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4"
                  checked={roleDraft.is_liquidity_provider}
                  onChange={() => toggleRole('is_liquidity_provider')}
                  disabled={savingRoles}
                />
                <div>
                  <p className="font-semibold text-foreground">Liquidity Provider</p>
                  <p className="text-sm text-muted-foreground">
                    Provide quotes and liquidity to Mosaic desks.
                  </p>
                </div>
              </label>

              <label className="flex items-start gap-3 rounded-lg border border-border/60 bg-muted/30 p-4 hover:border-primary transition-colors">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4"
                  checked={roleDraft.is_desk}
                  onChange={() => toggleRole('is_desk')}
                  disabled={savingRoles}
                />
                <div>
                  <p className="font-semibold text-foreground">Desk Manager</p>
                  <p className="text-sm text-muted-foreground">
                    Manage desks, monitor orders, and coordinate execution.
                  </p>
                </div>
              </label>
            </div>
          </div>
          <DialogFooter className="flex gap-2">
            <Button
              variant="outline"
              onClick={() => handleRoleModalChange(false)}
              disabled={savingRoles}
            >
              Not now
            </Button>
            <Button
              onClick={handleSaveRoles}
              disabled={savingRoles || !rolesChanged}
            >
              {savingRoles && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Save Roles
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
