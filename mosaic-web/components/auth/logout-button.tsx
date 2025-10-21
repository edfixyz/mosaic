'use client';

import { Button } from '@/components/ui/button';
import { resetMCPSession } from '@/lib/mcp-client';

export function LogoutButton() {
  const handleLogout = () => {
    resetMCPSession();
    window.location.href = '/auth/logout';
  };

  return (
    <Button variant="outline" onClick={handleLogout}>
      Log Out
    </Button>
  );
}
