"use client"

import Link from "next/link"
import Image from "next/image"
import { UserProfile } from "@/components/auth/user-profile"

export function Navigation() {

  return (
    <nav className="fixed top-0 left-0 right-0 z-50 border-b border-border bg-card">
      <div className="px-6">
        <div className="flex h-16 items-center justify-between">
          <Link href="/" className="flex items-center gap-3">
            <Image src="/mosaic_logo.png" alt="Mosaic Logo" width={32} height={32} className="object-contain" />
            <div className="flex flex-col">
              <div
                className="text-2xl font-serif text-primary leading-none"
                style={{ fontFamily: "var(--font-playfair)" }}
              >
                Mosaic
              </div>
              <span className="text-xs text-muted-foreground mt-0.5">by Edge Finance</span>
            </div>
          </Link>

          <div className="flex items-center gap-6">
            <Link href="/markets" className="text-sm text-foreground transition-colors hover:text-primary">
              All Markets
            </Link>
            <Link href="/assets" className="text-sm text-foreground transition-colors hover:text-primary">
              All Assets
            </Link>
            <UserProfile />
          </div>
        </div>
      </div>
    </nav>
  )
}
