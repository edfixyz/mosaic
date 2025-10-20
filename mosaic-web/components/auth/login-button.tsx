'use client';

import { Button } from '@/components/ui/button';

export function LoginButton() {
  return (
    <Button onClick={() => window.location.href = '/auth/login'}>
      Log In
    </Button>
  );
}
