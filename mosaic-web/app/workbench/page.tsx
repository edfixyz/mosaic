import { auth0 } from '@/lib/auth0'
import { redirect } from 'next/navigation'
import { WorkbenchClient } from './workbench-client'

export default async function WorkbenchPage() {
  const session = await auth0.getSession()

  if (!session) {
    redirect('/api/auth/login?returnTo=/workbench')
  }

  return (
    <div className="min-h-screen p-8">
      <div className="mb-8">
        <h1
          className="text-4xl font-serif mb-2 text-primary"
          style={{ fontFamily: 'var(--font-playfair)' }}
        >
          Workbench
        </h1>
        <p className="text-muted-foreground">
          Manage your assets and create new faucets
        </p>
      </div>

      <WorkbenchClient />
    </div>
  )
}
