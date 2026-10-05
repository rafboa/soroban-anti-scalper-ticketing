"use client";

import { useState } from "react";
import { ContractClient, SignFunction } from "@/lib/contractclient";

const FACE_VALUE   = Number(process.env.NEXT_PUBLIC_FACE_VALUE   ?? 100);
const MAX_MULT     = Number(process.env.NEXT_PUBLIC_MAX_MULT     ?? 110);
const PRICE_CEIL   = Math.floor(FACE_VALUE * MAX_MULT / 100);

interface Props {
  sellerPublicKey: string;
  signFn:          SignFunction;
  client:          ContractClient;
  tokenAddress:    string;
}

type Status = "idle" | "submitting" | "success" | "error";

export default function ResalePortal({
  sellerPublicKey,
  signFn,
  client,
  tokenAddress,
}: Props) {
  const [eventId,   setEventId]   = useState("1");
  const [ticketId,  setTicketId]  = useState("");
  const [buyerAddr, setBuyerAddr] = useState("");
  const [price,     setPrice]     = useState("");
  const [status,    setStatus]    = useState<Status>("idle");
  const [txHash,    setTxHash]    = useState<string | null>(null);
  const [errorMsg,  setErrorMsg]  = useState<string | null>(null);

  const priceNum     = Number(price);
  const priceInvalid = price !== "" && (isNaN(priceNum) || priceNum <= 0);
  const priceTooHigh = !priceInvalid && priceNum > PRICE_CEIL;

  async function handleSubmit() {
    if (!eventId || !ticketId || !buyerAddr || !price) return;
    if (priceInvalid || priceTooHigh) return;

    setStatus("submitting");
    setErrorMsg(null);
    setTxHash(null);

    try {
      const hash = await client.transferTicket(
        {
          eventId:      Number(eventId),
          ticketId:     Number(ticketId),
          from:         sellerPublicKey,
          to:           buyerAddr.trim(),
          amount:       BigInt(Math.round(priceNum)),
          tokenAddress,
        },
        sellerPublicKey,
        signFn,
      );

      setTxHash(hash);
      setStatus("success");
      setTicketId("");
      setBuyerAddr("");
      setPrice("");
    } catch (err: any) {
      setErrorMsg(err.message ?? "Transaction failed");
      setStatus("error");
    }
  }

  return (
    <section className="resale-portal" aria-label="Resale Portal">
      <h2>Transfer a Ticket</h2>
      <p className="hint">
        Price ceiling: <strong>{PRICE_CEIL} units</strong> ({MAX_MULT}% of {FACE_VALUE} face value).
        Transfers respect event holding quotas and velocity cooldowns.
      </p>

      <div className="form-group">
        <label htmlFor="event-id">Event ID</label>
        <input
          id="event-id"
          type="number"
          min={1}
          placeholder="e.g. 1"
          value={eventId}
          onChange={e => setEventId(e.target.value)}
          disabled={status === "submitting"}
        />
      </div>

      <div className="form-group">
        <label htmlFor="ticket-id">Ticket ID</label>
        <input
          id="ticket-id"
          type="number"
          min={1}
          placeholder="e.g. 42"
          value={ticketId}
          onChange={e => setTicketId(e.target.value)}
          disabled={status === "submitting"}
        />
      </div>

      <div className="form-group">
        <label htmlFor="buyer-addr">Buyer Wallet Address</label>
        <input
          id="buyer-addr"
          type="text"
          placeholder="G..."
          value={buyerAddr}
          onChange={e => setBuyerAddr(e.target.value)}
          disabled={status === "submitting"}
        />
      </div>

      <div className="form-group">
        <label htmlFor="price">
          Price (units)
          {priceTooHigh && (
            <span className="field-error" role="alert">
              {" "}— exceeds ceiling of {PRICE_CEIL}
            </span>
          )}
        </label>
        <input
          id="price"
          type="number"
          min={1}
          max={PRICE_CEIL}
          placeholder={`Max ${PRICE_CEIL}`}
          value={price}
          onChange={e => setPrice(e.target.value)}
          disabled={status === "submitting"}
          aria-invalid={priceTooHigh}
        />
      </div>

      <button
        className="btn-primary"
        onClick={handleSubmit}
        disabled={
          status === "submitting" ||
          !eventId || !ticketId || !buyerAddr || !price ||
          priceInvalid || priceTooHigh
        }
      >
        {status === "submitting" ? "Submitting…" : "Transfer Ticket"}
      </button>

      {status === "success" && txHash && (
        <div className="tx-success" role="status">
          <strong>Transfer complete!</strong>{" "}
          <a
            href={`https://stellar.expert/explorer/testnet/tx/${txHash}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            View on Stellar Expert ↗
          </a>
        </div>
      )}

      {status === "error" && errorMsg && (
        <div className="tx-error" role="alert">
          {errorMsg}
        </div>
      )}
    </section>
  );
}