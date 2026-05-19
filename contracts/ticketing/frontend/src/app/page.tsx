/**
 * StellarPass — Root Application Page
 * ────────────────────────────────────
 * Composes the three main UI sections:
 *   1. TicketGallery  — owned tickets
 *   2. ResalePortal   — transfer / sell
 *   3. CheckInQR      — gate entry QR
 */

"use client";

import { useState } from "react";
import { useWallet } from "@/lib/useWallet";
import { ContractClient } from "@/lib/ContractClient";
import TicketGallery from "@/components/TicketGallery";
import ResalePortal from "@/components/ResalePortal";
import CheckInQR from "@/components/CheckInQR";
import { Keypair } from "@stellar/stellar-sdk";

// ── Environment config (set in .env.local) ─────────────────────────────────

const CONTRACT_ID    = process.env.NEXT_PUBLIC_CONTRACT_ID    ?? "[Insert Deployed Testnet ID Here]";
const TOKEN_ADDRESS  = process.env.NEXT_PUBLIC_TOKEN_ADDRESS  ?? "";
const NETWORK        = (process.env.NEXT_PUBLIC_NETWORK as "testnet" | "mainnet") ?? "testnet";

const contractClient = ContractClient.forNetwork(CONTRACT_ID, NETWORK);

// ── Tabs ──────────────────────────────────────────────────────────────────

type Tab = "gallery" | "resale" | "checkin";

export default function HomePage() {
  const wallet       = useWallet();
  const [tab, setTab] = useState<Tab>("gallery");

  // In a real app the signer keypair comes from Freighter via signTransaction.
  // For local dev you can paste a testnet secret into .env.local.
  const devKeypair = process.env.NEXT_PUBLIC_DEV_SECRET
    ? Keypair.fromSecret(process.env.NEXT_PUBLIC_DEV_SECRET)
    : null;

  return (
    <main className="app">
      {/* ── Header ────────────────────────────────────────────────────── */}
      <header className="app-header">
        <div className="logo">
          <span className="logo-star">✦</span>
          <span className="logo-text">StellarPass</span>
        </div>

        {wallet.isConnected ? (
          <div className="wallet-badge">
            <span className="wallet-addr">
              {wallet.publicKey!.slice(0, 6)}…{wallet.publicKey!.slice(-4)}
            </span>
            <button className="btn-ghost" onClick={wallet.disconnect}>
              Disconnect
            </button>
          </div>
        ) : (
          <button
            className="btn-primary"
            onClick={wallet.connect}
            disabled={wallet.isConnecting}
          >
            {wallet.isConnecting ? "Connecting…" : "Connect Freighter"}
          </button>
        )}
      </header>

      {/* ── Wallet error ──────────────────────────────────────────────── */}
      {wallet.error && (
        <div className="wallet-error" role="alert">
          {wallet.error}
        </div>
      )}

      {/* ── Connect prompt ────────────────────────────────────────────── */}
      {!wallet.isConnected && !wallet.error && (
        <section className="connect-prompt">
          <h1>Fair tickets, on-chain.</h1>
          <p>
            Connect your Freighter wallet to view your tickets,
            resell them at a fair price, or generate your gate entry QR.
          </p>
        </section>
      )}

      {/* ── Main content ──────────────────────────────────────────────── */}
      {wallet.isConnected && wallet.publicKey && (
        <>
          {/* Tab navigation */}
          <nav className="tab-nav" aria-label="App sections">
            {(["gallery", "resale", "checkin"] as Tab[]).map(t => (
              <button
                key={t}
                className={`tab-btn ${tab === t ? "tab-btn--active" : ""}`}
                onClick={() => setTab(t)}
              >
                {{ gallery: "🎟 My Tickets", resale: "💸 Resale", checkin: "📲 Check-In" }[t]}
              </button>
            ))}
          </nav>

          {/* Tab panels */}
          <div className="tab-content">
            {tab === "gallery" && (
              <TicketGallery
                publicKey={wallet.publicKey}
                client={contractClient}
              />
            )}

            {tab === "resale" && devKeypair && (
              <ResalePortal
                sellerPublicKey={wallet.publicKey}
                signerKeypair={devKeypair}
                client={contractClient}
                tokenAddress={TOKEN_ADDRESS}
              />
            )}

            {tab === "resale" && !devKeypair && (
              <div className="notice">
                Set <code>NEXT_PUBLIC_DEV_SECRET</code> in <code>.env.local</code> to
                enable transaction signing in dev mode. In production, this routes
                through Freighter.
              </div>
            )}

            {tab === "checkin" && (
              <CheckInQR
                ticketId={1}
                ownerPublicKey={wallet.publicKey}
                signMessage={wallet.signTransaction}
              />
            )}
          </div>
        </>
      )}
    </main>
  );
}