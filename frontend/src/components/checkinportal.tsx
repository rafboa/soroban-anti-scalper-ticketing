"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import QRCode from "qrcode";

const QR_TTL_SECONDS = 60;

interface CheckInPayload {
  eventId:   number;
  ticketId:  number;
  owner:     string;
  expiry:    number;
  nonce:     string;
  signature: string;
}

interface Props {
  eventId:        number;
  ticketId:       number;
  ownerPublicKey: string;
  signMessage:    (message: string) => Promise<string>;
}

type QRState = "idle" | "generating" | "ready" | "expired" | "error";

export default function CheckInQR({ eventId, ticketId, ownerPublicKey, signMessage }: Props) {
  const [qrState,     setQRState]     = useState<QRState>("idle");
  const [dataUrl,     setDataUrl]     = useState<string | null>(null);
  const [expiresAt,   setExpiresAt]   = useState<number | null>(null);
  const [secondsLeft, setSecondsLeft] = useState<number>(0);
  const [errorMsg,    setErrorMsg]    = useState<string | null>(null);
  const timerRef                      = useRef<ReturnType<typeof setInterval> | null>(null);

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

  const generate = useCallback(async () => {
    setQRState("generating");
    setErrorMsg(null);

    try {
      const expiry    = Math.floor(Date.now() / 1000) + QR_TTL_SECONDS;
      const nonce     = crypto.getRandomValues(new Uint8Array(8))
                              .reduce((h, b) => h + b.toString(16).padStart(2, "0"), "");
      const message   = canonicalMessage(eventId, ticketId, ownerPublicKey, expiry, nonce);
      const signature = await signMessage(message);

      const payload: CheckInPayload = {
        eventId, ticketId, owner: ownerPublicKey, expiry, nonce, signature,
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
  }, [eventId, ticketId, ownerPublicKey, signMessage]);

  return (
    <section className="checkin-qr" aria-label="Check-In QR Code">
      <h2>Gate Entry QR</h2>
      <p className="hint">
        Event <strong>#{eventId}</strong>, Ticket <strong>#{String(ticketId).padStart(4, "0")}</strong>
        <br/>QR codes expire after {QR_TTL_SECONDS}s.
      </p>

      {qrState === "idle" && (
        <button className="btn-primary" onClick={generate}>
          Generate Check-In QR
        </button>
      )}

      {qrState === "generating" && (
        <div className="qr-loading" role="status" aria-live="polite">
          <span className="spinner" aria-hidden="true" />
          <span>Requesting wallet signature…</span>
        </div>
      )}

      {qrState === "ready" && dataUrl && (
        <div className="qr-wrapper">
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
        <div className="qr-expired" role="status">
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

function canonicalMessage(
  eventId:  number,
  ticketId: number,
  owner:    string,
  expiry:   number,
  nonce:    string,
): string {
  return `stellarpass:checkin:${eventId}:${ticketId}:${owner}:${expiry}:${nonce}`;
}