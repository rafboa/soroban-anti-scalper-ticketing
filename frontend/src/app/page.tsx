"use client";

import { useState } from "react";
import { useWallet } from "@/lib/useWallet";
import { ContractClient } from "@/lib/contractclient";
import TicketGallery from "@/components/ticketgallery";
import CheckInQR from "@/components/checkinportal";
import ResalePortal from "@/components/resaleportal";

const CONTRACT_ID   = process.env.NEXT_PUBLIC_CONTRACT_ID   ?? "[Insert Deployed Testnet ID Here]";
const NETWORK       = (process.env.NEXT_PUBLIC_NETWORK as "testnet" | "mainnet") ?? "testnet";
const DEV_WALLET    = process.env.NEXT_PUBLIC_DEV_WALLET    ?? "";
const TOKEN_ADDRESS = process.env.NEXT_PUBLIC_TOKEN_ADDRESS ?? "";

const contractClient = ContractClient.forNetwork(CONTRACT_ID, NETWORK);

type Tab = "gallery" | "resale" | "checkin";

export default function HomePage() {
  const wallet        = useWallet();
  const [tab, setTab] = useState<Tab>("gallery");

  const isDevMode   = !!DEV_WALLET;
  const activeKey   = isDevMode ? DEV_WALLET : wallet.publicKey;
  const isConnected = isDevMode ? true       : wallet.isConnected;

  const devSign = async (xdr: string) => xdr;
  const signFn  = isDevMode ? devSign : wallet.signTransaction;

  return (
    <main className="app">
      <header className="app-header">
        <div className="logo">
          <span className="logo-star" aria-hidden="true">✦</span>
          <span className="logo-text">StellarPass</span>
        </div>

        {isDevMode ? (
          <div className="wallet-badge">
            <span className="dev-badge">DEV MODE</span>
            <span className="wallet-addr">
              {DEV_WALLET.slice(0, 6)}…{DEV_WALLET.slice(-4)}
            </span>
          </div>
        ) : isConnected ? (
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

      {isDevMode && (
        <div className="notice" style={{ marginBottom: "1.5rem" }}>
          Running in dev mode with wallet <code>{DEV_WALLET.slice(0, 6)}…{DEV_WALLET.slice(-4)}</code>.
          Remove <code>NEXT_PUBLIC_DEV_WALLET</code> from <code>.env.local</code> to use Freighter.
        </div>
      )}

      {!isDevMode && wallet.error && (
        <div className="wallet-error" role="alert">
          {wallet.error}
        </div>
      )}

      {!isConnected && !wallet.error && !isDevMode && (
        <section className="connect-prompt">
          <h1>Fair tickets, on-chain.</h1>
          <p>
            Connect your Freighter wallet to view your tickets,
            resell them at a fair price, or generate your gate entry QR.
          </p>
        </section>
      )}

      {isConnected && activeKey && (
        <>
          <nav className="tab-nav" aria-label="Application sections" role="tablist">
            {(["gallery", "resale", "checkin"] as Tab[]).map(t => {
              const tabLabels: Record<Tab, string> = {
                gallery: "My Tickets",
                resale:  "Resale",
                checkin: "Check-In",
              };
              const isSelected = tab === t;
              return (
                <button
                  key={t}
                  role="tab"
                  aria-selected={isSelected}
                  className={`tab-btn ${isSelected ? "tab-btn--active" : ""}`}
                  onClick={() => setTab(t)}
                >
                  {tabLabels[t]}
                </button>
              );
            })}
          </nav>

          <div className="tab-content" role="tabpanel">
            {tab === "gallery" && (
              <TicketGallery
                publicKey={activeKey}
                client={contractClient}
              />
            )}

            {tab === "resale" && (
              <ResalePortal
                sellerPublicKey={activeKey}
                signFn={signFn}
                client={contractClient}
                tokenAddress={TOKEN_ADDRESS}
              />
            )}

            {tab === "checkin" && (
              <CheckInQR
                eventId={1}
                ticketId={1}
                ownerPublicKey={activeKey}
                signMessage={signFn}
              />
            )}
          </div>
        </>
      )}
    </main>
  );
}