'use client';

import { useEffect, useState } from 'react';
import { LoginButton } from './login-button';
import { LogoutButton } from './logout-button';
import { Button } from '@/components/ui/button';
import { ensureMCPConnection, resetMCPSession } from '@/lib/mcp-client';
import Link from 'next/link'

interface User {
  email?: string;
  name?: string;
}

export function UserProfile() {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    fetch('/api/auth/me')
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        setUser(data);
        setIsLoading(false);
      })
      .catch(() => setIsLoading(false));
  }, []);

  useEffect(() => {
    if (!user) {
      return;
    }

    let cancelled = false;

    (async () => {
      try {
        const tokenRes = await fetch('/api/auth/token');
        if (!tokenRes.ok) {
          return;
        }

        const tokenData = await tokenRes.json();
        if (!tokenData.accessToken || cancelled) {
          return;
        }

        await ensureMCPConnection(tokenData.accessToken);
      } catch (error) {
        console.error('❌ Failed to pre-connect to MCP:', error);
      }
    })();

    return () => {
      cancelled = true;
      resetMCPSession().catch((error) =>
        console.warn('Failed to reset MCP session on unmount', error)
      );
    };
  }, [user]);

  if (isLoading) return <div>Loading...</div>;

  if (user) {
    return (
      <div className="flex items-center gap-4">
        <span className="text-sm text-muted-foreground">
          {user.email || user.name}
        </span>
        <Button variant="outline" asChild>
          <Link href="/settings">Settings</Link>
        </Button>
        <LogoutButton />
      </div>
    );
  }

  return <LoginButton />;
}
