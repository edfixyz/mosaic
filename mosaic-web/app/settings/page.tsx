import { auth0 } from '@/lib/auth0'
import { redirect } from 'next/navigation'
import { SettingsClient } from './settings-client'

export default async function SettingsPage() {
  const session = await auth0.getSession()

  if (!session) {
    redirect('/api/auth/login?returnTo=/settings')
  }

  return (
    <div className="min-h-screen p-8">
      <div className="mb-8">
        <h1
          className="text-4xl font-serif mb-2 text-primary"
          style={{ fontFamily: 'var(--font-playfair)' }}
        >
          Settings
        </h1>
        <p className="text-muted-foreground">
          Manage your desks, assets, and market access configuration.
        </p>
      </div>

      <SettingsClient />
    </div>
  )
}
