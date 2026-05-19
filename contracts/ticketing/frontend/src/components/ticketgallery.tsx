/**
 * TicketGallery
 * ─────────────
 * Displays all tickets owned by the connected wallet.
 * Fetches ticket IDs from a lightweight off-chain index API and hydrates
 * ownership + usage state from the on-chain ContractClient.
 */

"use client";

import { useEffect, useState } from "react";
import { ContractClient, TicketRecord } from "@/lib/ContractClient";

interface OwnedTicket {
  id:     number;
  record: TicketRecord;
}

interface Props {
  publicKey:  string;
  client:     ContractClient;
}

export default function TicketGallery({ publicKey, client }: Props) {
  const [tickets,  setTickets]  = useState<OwnedTicket[]>([]);
  const [loading,  setLoading]  = useState(true);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setErrorMsg(null);
      try {
        // Fetch the list of ticket IDs owned by this wallet from the
        // off-chain indexer.  Fall back to empty array on failure.
        const res = await fetch(
          `/api/tickets?owner=${encodeURIComponent(publicKey)}`
        );
        const ids: number[] = res.ok ? await res.json() : [];

        const hydrated = await Promise.all(
          ids.map(async (id) => {
            const record = await client.getTicket(id);
            return record ? { id, record } : null;
          })
        );

        setTickets(hydrated.filter(Boolean) as OwnedTicket[]);
      } catch (err: any) {
        setErrorMsg(err.message);
      } finally {
        setLoading(false);
      }
    }

    load();
  }, [publicKey, client]);

  if (loading) {
    return (
      <div className="gallery-loading">
        <span className="spinner" />
        Loading your tickets…
      </div>
    );
  }

  if (errorMsg) {
    return <div className="gallery-error">Error: {errorMsg}</div>;
  }

  if (tickets.length === 0) {
    return (
      <div className="gallery-empty">
        <p>No tickets found for this wallet.</p>
        <p className="hint">Purchase tickets at the event box office.</p>
      </div>
    );
  }

  return (
    <section className="gallery" aria-label="Your Tickets">
      <h2>Your Tickets</h2>
      <div className="ticket-grid">
        {tickets.map(({ id, record }) => (
          <TicketCard key={id} ticketId={id} record={record} />
        ))}
      </div>
    </section>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// TicketCard
// ─────────────────────────────────────────────────────────────────────────────

function TicketCard({ ticketId, record }: { ticketId: number; record: TicketRecord }) {
  const statusLabel = record.is_used ? "USED" : "VALID";
  const statusClass = record.is_used ? "status-used" : "status-valid";

  return (
    <article className={`ticket-card ${record.is_used ? "ticket-card--used" : ""}`}>
      <header className="ticket-card__header">
        <span className="ticket-card__id">#{String(ticketId).padStart(4, "0")}</span>
        <span className={`ticket-card__status ${statusClass}`}>{statusLabel}</span>
      </header>

      <div className="ticket-card__owner">
        <span className="label">Owner</span>
        <code>{record.owner.slice(0, 6)}…{record.owner.slice(-4)}</code>
      </div>

      {record.is_used && (
        <div className="ticket-card__used-badge" aria-label="Ticket has been used">
          ✓ Checked in
        </div>
      )}
    </article>
  );
}