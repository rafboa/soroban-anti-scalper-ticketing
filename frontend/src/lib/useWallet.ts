"use client";

import { useState, useCallback } from "react";
import {
  isConnected,
  getAddress,
  signTransaction,
} from "@stellar/freighter-api";

export interface WalletState {
  publicKey:       string | null;
  isConnected:     boolean;
  isConnecting:    boolean;
  error:           string | null;
  connect:         () => Promise<void>;
  disconnect:      () => void;
  signTransaction: (xdr: string) => Promise<string>;
}

export function useWallet(): WalletState {
  const [publicKey,    setPublicKey]    = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error,        setError]        = useState<string | null>(null);

  const connect = useCallback(async () => {
    setError(null);
    setIsConnecting(true);
    try {
      const connected = await isConnected();
      if (!connected) {
        throw new Error("Freighter is not connected. Open and unlock it first.");
      }
      const { address } = await getAddress();
      if (!address) throw new Error("Could not get address from Freighter.");
      setPublicKey(address);
    } catch (err: any) {
      setError(err.message ?? "Unknown wallet error");
    } finally {
      setIsConnecting(false);
    }
  }, []);

  const disconnect = useCallback(() => {
    setPublicKey(null);
    setError(null);
  }, []);

  const sign = useCallback(async (xdr: string): Promise<string> => {
    const result = await signTransaction(xdr, {
      networkPassphrase: process.env.NEXT_PUBLIC_NETWORK_PASSPHRASE,
    });
    return result.signedTxXdr;
  }, []);

  return {
    publicKey,
    isConnected:     !!publicKey,
    isConnecting,
    error,
    connect,
    disconnect,
    signTransaction: sign,
  };
}