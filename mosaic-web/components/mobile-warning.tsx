"use client"

import { useEffect, useState } from "react"
import { Card } from "@/components/ui/card"
import { Monitor } from "lucide-react"

export function MobileWarning() {
  const [isMobile, setIsMobile] = useState(false)

  useEffect(() => {
    const checkScreenSize = () => {
      // Consider screens smaller than 1024px as mobile/tablet
      setIsMobile(window.innerWidth < 1024)
    }

    // Check on mount
    checkScreenSize()

    // Add event listener for window resize
    window.addEventListener("resize", checkScreenSize)

    return () => window.removeEventListener("resize", checkScreenSize)
  }, [])

  if (!isMobile) return null

  return (
    <div className="fixed inset-0 z-[100] bg-background flex items-center justify-center p-6">
      <Card className="max-w-md w-full p-8 bg-card border-border text-center">
        <div className="flex justify-center mb-6">
          <Monitor className="h-16 w-16 text-primary" />
        </div>
        <h1
          className="text-3xl font-serif text-primary mb-4"
          style={{ fontFamily: "var(--font-playfair)" }}
        >
          Desktop Only
        </h1>
        <p className="text-muted-foreground mb-4">
          Mosaic is not yet optimized for mobile or small screen sizes.
        </p>
        <p className="text-sm text-muted-foreground">
          Please access this application on a desktop or laptop computer for the best experience.
        </p>
      </Card>
    </div>
  )
}
