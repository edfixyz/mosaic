import type React from "react"
import type { Metadata } from "next"
import { GeistSans } from "geist/font/sans"
import { GeistMono } from "geist/font/mono"
import { Playfair_Display, DM_Mono } from "next/font/google"
import { Analytics } from "@vercel/analytics/next"
import "./globals.css"
import { Navigation } from "@/components/navigation"
import { Sidebar } from "@/components/sidebar"
import { MobileWarning } from "@/components/mobile-warning"
import { Suspense } from "react"

const playfair = Playfair_Display({
  subsets: ["latin"],
  variable: "--font-playfair",
  display: "swap",
})

const dmMono = DM_Mono({
  subsets: ["latin"],
  weight: ["300", "400", "500"],
  variable: "--font-dm-mono",
  display: "swap",
})

export const metadata: Metadata = {
  title: "Mosaic - Edge Finance OTC Desk",
  description: "Private OTC trading desk - Private Trades, Public Trust"
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <body className={`font-sans ${GeistSans.variable} ${GeistMono.variable} ${playfair.variable} ${dmMono.variable}`}>
        <div className="fixed top-0 left-0 right-0 border border-red-600 text-red-600 text-sm py-2 z-[60] bg-card overflow-hidden">
          <div className="inline-block whitespace-nowrap animate-marquee">
            This product is in alpha • Expect bugs and dragons • Follow @edfixy
          </div>
        </div>
        <MobileWarning />
        <Suspense fallback={<div>Loading...</div>}>
          <Navigation />
          <Sidebar />
          <main className="ml-64 mt-[92px]">{children}</main>
        </Suspense>
        <Analytics />
      </body>
    </html>
  )
}
