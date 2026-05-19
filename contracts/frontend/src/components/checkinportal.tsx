/**
 * CheckInQR
 * ─────────
 * Generates a time-limited, cryptographically signed QR code that the gate
 * manager app can scan to trigger `check_in` on-chain.
 *
 * ## How it works
 * 1. The holder selects their ticket.
 * 2. The component builds a signed payload:
 *      { ticketId, owner, expiry, signature }
 *    where `signature` is the holder's Freighter signature over the
 *    canonical message string.
 * 3. The payload is encoded as a QR code.
 * 4. The QR expires after 60 seconds — screenshots are useless because the
 *    gate app rejects stale payloads, and the actual on-chain `check_in`
 *    still requires a fresh wallet signature at submission time.
 *
 * NOTE: In production the gate app submits `check_in` using a dedicated
 * operator keypair after verifying the QR payload off-chain.  The on-chain
 * auth requirement for the ticket *owner* is enforced via pre-authorisation
 * (a `SorobanAuthorizedEntry` embedded in the QR payload in a full
 * implementation).  This component demonstrates the UX layer.
 */

"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import QRCode from "qrcode";

const QR_TTL_SECONDS = 60;

interface CheckInPayload {
  ticketId:  number;
  owner:     string;
  expiry:    number;        // Unix timestamp (seconds)
  nonce:     string;        // random hex to prevent replay within the window
  signature: string;        // Freighter signature over canonical message
}

interface Props {
  ticketId:        number;
  ownerPublicKey:  string;
  /** signMessage — provided by the useWallet hook (routes through Freighter) */
  signMessage:     (message: string) => Promise<string>;
}

type QRState = "idle" | "generating" | "ready" | "expired" | "error";

export default function CheckInQR({ ticketId, ownerPublicKey, signMessage }: Props) {
  const [qrState,    setQRState]    = useState<QRState>("idle");
  const [dataUrl,    setDataUrl]    = useState<string | null>(null);
  const [expiresAt,  setExpiresAt]  = useState<number | null>(null);
  const [secondsLeft, setSecondsLeft] = useState<number>(0);
  const [errorMsg,   setErrorMsg]   = useState<string | null>(null);
  const timerRef                    = useRef<ReturnType<typeof setInterval> | null>(null);

  // ── Countdown timer ────────────────────────────────────────────────────────

  useEffect(() => {
    if (qrState !== "ready" || expiresAt === null) return;

    timerRef.current = setInterval(() => {
      const remaining = Math.max(0, expiresAt - Math.floor(Date.now() / 1000));
      setSecondsLeft(remaining);
      if (remaining === 0) {
        setQRState("expired");
        setDataUrl(null);
        clearInterval(timerRef.current!);
      }
    }, 1_000);

    return () => clearInterval(timerRef.current!);
  }, [qrState, expiresAt]);

  // ── Generate QR ────────────────────────────────────────────────────────────

  const generate = useCallback(async () => {
    setQRState("generating");
    setErrorMsg(null);

    try {
      const expiry    = Math.floor(Date.now() / 1000) + QR_TTL_SECONDS;
      const nonce     = crypto.getRandomValues(new Uint8Array(8))
                              .reduce((h, b) => h + b.toString(16).padStart(2, "0"), "");
      const message   = canonicalMessage(ticketId, ownerPublicKey, expiry, nonce);
      const signature = await signMessage(message);

      const payload: CheckInPayload = {
        ticketId, owner: ownerPublicKey, expiry, nonce, signature,
      };

      const url = await QRCode.toDataURL(JSON.stringify(payload), {
        errorCorrectionLevel: "H",
        width:                320,
        margin:               2,
        color: { dark: "#0f1117", light: "#ffffff" },
      });

      setDataUrl(url);
      setExpiresAt(expiry);
      setSecondsLeft(QR_TTL_SECONDS);
      setQRState("ready");
    } catch (err: any) {
      setErrorMsg(err.message ?? "Failed to generate QR code");
      setQRState("error");
    }
  }, [ticketId, ownerPublicKey, signMessage]);

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <section className="checkin-qr" aria-label="Check-In QR Code">
      <h2>Gate Entry QR</h2>
      <p className="hint">
        Ticket <strong>#{String(ticketId).padStart(4, "0")}</strong> —
        QR codes expire after {QR_TTL_SECONDS}s.
      </p>

      {qrState === "idle" && (
        <button className="btn-primary" onClick={generate}>
          Generate Check-In QR
        </button>
      )}

      {qrState === "generating" && (
        <div className="qr-loading">
          <span className="spinner" />
          Requesting wallet signature…
        </div>
      )}

      {qrState === "ready" && dataUrl && (
        <div className="qr-wrapper">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src={dataUrl}
            alt={`Check-in QR for ticket #${ticketId}`}
            className="qr-image"
            width={320}
            height={320}
          />
          <div className="qr-countdown" aria-live="polite">
            Expires in <strong>{secondsLeft}s</strong>
          </div>
          <button className="btn-secondary" onClick={generate}>
            Regenerate
          </button>
        </div>
      )}

      {qrState === "expired" && (
        <div className="qr-expired">
          <p>QR code expired.</p>
          <button className="btn-primary" onClick={generate}>
            Generate New QR
          </button>
        </div>
      )}

      {qrState === "error" && errorMsg && (
        <div className="qr-error" role="alert">
          {errorMsg}
          <button className="btn-secondary" onClick={generate}>
            Retry
          </button>
        </div>
      )}
    </section>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Deterministic canonical message string that both the QR generator and the
 * gate verifier must agree on.
 */
function canonicalMessage(
  ticketId: number,
  owner:    string,
  expiry:   number,
  nonce:    string,
): string {
  return `stellarpass:checkin:${ticketId}:${owner}:${expiry}:${nonce}`;
}