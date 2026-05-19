/**
 * ContractClient
 * ──────────────
 * Thin wrapper around @stellar/stellar-sdk that encodes / decodes
 * StellarPass contract invocations and handles transaction submission.
 *
 * Usage:
 *   const client = new ContractClient(contractId, rpcUrl, networkPassphrase);
 *   const ticket = await client.getTicket(ticketId);
 */

import {
  Contract,
  Keypair,
  Networks,
  nativeToScVal,
  scValToNative,
  SorobanRpc,
  TransactionBuilder,
  BASE_FEE,
  xdr,
} from "@stellar/stellar-sdk";

// ─────────────────────────────────────────────────────────────────────────────
// Types mirroring the on-chain TicketRecord struct
// ─────────────────────────────────────────────────────────────────────────────

export interface TicketRecord {
  owner: string;
  is_used: boolean;
}

export interface TransferParams {
  ticketId: number;
  from: string;
  to: string;
  amount: bigint;
  tokenAddress: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Network presets
// ─────────────────────────────────────────────────────────────────────────────

export const NETWORKS = {
  testnet: {
    rpcUrl:            "https://soroban-testnet.stellar.org",
    networkPassphrase: Networks.TESTNET,
  },
  mainnet: {
    rpcUrl:            "https://mainnet.sorobanrpc.com",
    networkPassphrase: Networks.PUBLIC,
  },
} as const;

export type NetworkName = keyof typeof NETWORKS;

// ─────────────────────────────────────────────────────────────────────────────
// ContractClient
// ─────────────────────────────────────────────────────────────────────────────

export class ContractClient {
  private contract:          Contract;
  private server:            SorobanRpc.Server;
  private networkPassphrase: string;

  constructor(
    contractId:        string,
    rpcUrl:            string,
    networkPassphrase: string,
  ) {
    this.contract          = new Contract(contractId);
    this.server            = new SorobanRpc.Server(rpcUrl, { allowHttp: false });
    this.networkPassphrase = networkPassphrase;
  }

  // ── Factory for named networks ────────────────────────────────────────────

  static forNetwork(contractId: string, network: NetworkName): ContractClient {
    const { rpcUrl, networkPassphrase } = NETWORKS[network];
    return new ContractClient(contractId, rpcUrl, networkPassphrase);
  }

  // ── Read-only helpers ──────────────────────────────────────────────────────

  /**
   * Fetch the current owner and usage state for a ticket.
   * Returns `null` if the ticket does not exist on-chain.
   */
  async getTicket(ticketId: number): Promise<TicketRecord | null> {
    const operation = this.contract.call(
      "get_ticket",
      nativeToScVal(ticketId, { type: "u32" }),
    );

    const result = await this.server.simulateTransaction(
      await this._buildTx(operation),
    );

    if (SorobanRpc.Api.isSimulationError(result)) {
      throw new Error(`Simulation error: ${result.error}`);
    }

    const returnVal = (result as SorobanRpc.Api.SimulateTransactionSuccessResponse)
      .result?.retval;

    if (!returnVal || returnVal.switch() === xdr.ScValType.scvVoid()) {
      return null;
    }

    const native = scValToNative(returnVal) as { owner: string; is_used: boolean };
    return { owner: native.owner, is_used: native.is_used };
  }

  // ── Signed transaction builders ───────────────────────────────────────────

  /**
   * Build, simulate, sign, and submit a `transfer_ticket` transaction.
   *
   * The caller must supply a Keypair for the `from` account (the seller).
   * The buyer must have already set a token allowance for this contract.
   */
  async transferTicket(
    params:     TransferParams,
    signerKeys: Keypair,
  ): Promise<string> {
    const operation = this.contract.call(
      "transfer_ticket",
      nativeToScVal(params.ticketId,      { type: "u32"     }),
      nativeToScVal(params.from,          { type: "address" }),
      nativeToScVal(params.to,            { type: "address" }),
      nativeToScVal(params.amount,        { type: "i128"    }),
      nativeToScVal(params.tokenAddress,  { type: "address" }),
    );

    return this._signAndSubmit(operation, signerKeys);
  }

  /**
   * Build, simulate, sign, and submit a `check_in` transaction.
   * The holder's Keypair is required to satisfy `owner.require_auth()`.
   */
  async checkIn(
    ticketId:   number,
    owner:      string,
    signerKeys: Keypair,
  ): Promise<string> {
    const operation = this.contract.call(
      "check_in",
      nativeToScVal(ticketId, { type: "u32"     }),
      nativeToScVal(owner,    { type: "address" }),
    );

    return this._signAndSubmit(operation, signerKeys);
  }

  // ── Private helpers ────────────────────────────────────────────────────────

  private async _buildTx(operation: xdr.Operation): Promise<any> {
    // A dummy keypair is fine for simulation-only calls.
    const dummyKeypair = Keypair.random();
    const account      = await this.server.getAccount(dummyKeypair.publicKey()).catch(
      () => ({ id: dummyKeypair.publicKey(), sequence: "0" }),
    );

    return new TransactionBuilder(account as any, {
      fee:               BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();
  }

  private async _signAndSubmit(
    operation:  xdr.Operation,
    signerKeys: Keypair,
  ): Promise<string> {
    const account = await this.server.getAccount(signerKeys.publicKey());

    let tx = new TransactionBuilder(account, {
      fee:               BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();

    // Simulate to get the Soroban footprint / auth.
    const simResult = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simResult)) {
      throw new Error(`Simulation failed: ${simResult.error}`);
    }

    // Assemble (injects footprint + auth entries) and sign.
    const assembled = SorobanRpc.assembleTransaction(tx, simResult).build();
    assembled.sign(signerKeys);

    const sendResult = await this.server.sendTransaction(assembled);
    if (sendResult.status === "ERROR") {
      throw new Error(`Submission failed: ${JSON.stringify(sendResult.errorResult)}`);
    }

    // Poll until final status.
    let getResult = await this.server.getTransaction(sendResult.hash);
    while (getResult.status === SorobanRpc.Api.GetTransactionStatus.NOT_FOUND) {
      await new Promise(r => setTimeout(r, 1_000));
      getResult = await this.server.getTransaction(sendResult.hash);
    }

    if (getResult.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
      throw new Error("Transaction failed on-chain.");
    }

    return sendResult.hash;
  }
}