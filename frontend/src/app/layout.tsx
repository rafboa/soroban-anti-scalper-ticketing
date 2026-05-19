import type { Metadata } from "next";
// TypeScript may not have declarations for importing CSS files in this project setup.
// Ignore the missing module/type declarations for this side-effect import.
// @ts-ignore
import "./globals.css";

export const metadata: Metadata = {
  title: "StellarPass — Anti-Scalper Ticketing",
  description: "Decentralized event ticketing on Stellar.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}