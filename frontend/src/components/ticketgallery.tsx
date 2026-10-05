"use client";

import { useEffect, useState } from "react";
import { ContractClient, TicketRecord } from "@/lib/contractclient";

interface OwnedTicket {
  eventId: number;
  id:      number;
  record:  TicketRecord;
}

interface Props {
  publicKey: string;
  client:    ContractClient;
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
        const res = await fetch(
          `/api/tickets?owner=${encodeURIComponent(publicKey)}`
        );
        const ids: number[] = res.ok ? await res.json() : [];

        const hydrated = await Promise.all(
          ids.map(async (id) => {
            const record = await client.getTicket(1, id);
            return record ? { eventId: 1, id, record } : null;
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
      <div className="gallery-loading" role="status" aria-live="polite">
        <span className="spinner" aria-hidden="true" />
        <span>Loading your tickets…</span>
      </div>
    );
  }

  if (errorMsg) {
    return <div className="gallery-error" role="alert">Error: {errorMsg}</div>;
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
        {tickets.map(({ eventId, id, record }) => (
          <TicketCard key={`${eventId}-${id}`} ticketId={id} record={record} />
        ))}
      </div>
    </section>
  );
}

function TicketCard({ ticketId, record }: { ticketId: number; record: TicketRecord }) {
  const statusLabel = record.is_refunded ? "REFUNDED" : record.is_used ? "USED" : "VALID";
  const statusClass = record.is_refunded ? "status-refunded" : record.is_used ? "status-used" : "status-valid";

  return (
    <article className={`ticket-card ${record.is_used ? "ticket-card--used" : ""}`}>
      <header className="ticket-card__header">
        <span className="ticket-card__title">{record.title || "Stellar Event"}</span>
        <span className="ticket-card__id">#{String(ticketId).padStart(4, "0")}</span>
        <span className={`ticket-card__status ${statusClass}`}>{statusLabel}</span>
      </header>

      <div className="ticket-card__metadata">
        <div><strong>Venue:</strong> {record.venue}</div>
        <div><strong>Date:</strong> {new Date(Number(record.date) * 1000).toLocaleDateString()}</div>
        <div><strong>Seat:</strong> {record.seat}</div>
      </div>

      <div className="ticket-card__owner">
        <span className="label">Owner</span>
        <code>{record.owner.slice(0, 6)}…{record.owner.slice(-4)}</code>
      </div>
    </article>
  );
}