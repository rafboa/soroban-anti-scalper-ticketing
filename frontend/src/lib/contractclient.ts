import {
  SorobanRpc,
  TransactionBuilder,
  xdr,
  scValToNative,
  nativeToScVal,
  Contract,
  Keypair,
  Transaction,
} from "@stellar/stellar-sdk";

const BASE_FEE = "100000";

export interface TicketRecord {
  owner:                   string;
  is_used:                 boolean;
  is_refunded:             boolean;
  title:                   string;
  venue:                   string;
  date:                    number;
  seat:                    string;
  last_transfer_timestamp: number;
}

export interface EventConfig {
  admin:                     string;
  payment_token:             string;
  face_value:                bigint;
  max_resale_multiplier:     number;
  royalty_basis_points:      number;
  royalty_recipient:         string;
  whitelist_enabled:         boolean;
  max_supply:                number;
  current_supply:            number;
  max_tickets_per_wallet:    number;
  transfer_cooldown_seconds: bigint;
  is_paused:                 boolean;
}

export interface TransferParams {
  eventId:      number;
  ticketId:     number;
  from:         string;
  to:           string;
  amount:       bigint;
  tokenAddress: string;
}

export const NETWORKS = {
  testnet: {
    rpcUrl:            "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
  },
  mainnet: {
    rpcUrl:            "https://mainnet.sorobanrpc.com",
    networkPassphrase: "Public Global Stellar Network ; September 2015",
  },
} as const;

export type NetworkName = keyof typeof NETWORKS;

export type SignFunction = (xdr: string) => Promise<string>;

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

  static forNetwork(contractId: string, network: NetworkName): ContractClient {
    const { rpcUrl, networkPassphrase } = NETWORKS[network];
    return new ContractClient(contractId, rpcUrl, networkPassphrase);
  }

  async getTicket(eventId: number, ticketId: number): Promise<TicketRecord | null> {
    const operation = this.contract.call(
      "get_ticket",
      nativeToScVal(eventId, { type: "u32" }),
      nativeToScVal(ticketId, { type: "u32" }),
    );

    const dummyKeypair = Keypair.random();
    let account;
    try {
      account = await this.server.getAccount(dummyKeypair.publicKey());
    } catch {
      account = { id: dummyKeypair.publicKey(), sequence: "0" } as any;
    }

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();

    const result = await this.server.simulateTransaction(tx);

    if (SorobanRpc.Api.isSimulationError(result)) {
      throw new Error(`Simulation error: ${result.error}`);
    }

    const returnVal = (result as SorobanRpc.Api.SimulateTransactionSuccessResponse)
      .result?.retval;

    if (!returnVal || returnVal.switch() === xdr.ScValType.scvVoid()) {
      return null;
    }

    return scValToNative(returnVal) as TicketRecord;
  }

  async getEvent(eventId: number): Promise<EventConfig | null> {
    const operation = this.contract.call(
      "get_event",
      nativeToScVal(eventId, { type: "u32" }),
    );

    const dummyKeypair = Keypair.random();
    let account;
    try {
      account = await this.server.getAccount(dummyKeypair.publicKey());
    } catch {
      account = { id: dummyKeypair.publicKey(), sequence: "0" } as any;
    }

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();

    const result = await this.server.simulateTransaction(tx);

    if (SorobanRpc.Api.isSimulationError(result)) {
      throw new Error(`Simulation error: ${result.error}`);
    }

    const returnVal = (result as SorobanRpc.Api.SimulateTransactionSuccessResponse)
      .result?.retval;

    if (!returnVal || returnVal.switch() === xdr.ScValType.scvVoid()) {
      return null;
    }

    return scValToNative(returnVal) as EventConfig;
  }

  async getUserTicketBalance(eventId: number, userAddress: string): Promise<number> {
    const operation = this.contract.call(
      "get_user_ticket_balance",
      nativeToScVal(eventId,     { type: "u32"     }),
      nativeToScVal(userAddress, { type: "address" }),
    );

    const dummyKeypair = Keypair.random();
    let account;
    try {
      account = await this.server.getAccount(dummyKeypair.publicKey());
    } catch {
      account = { id: dummyKeypair.publicKey(), sequence: "0" } as any;
    }

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();

    const result = await this.server.simulateTransaction(tx);

    if (SorobanRpc.Api.isSimulationError(result)) {
      throw new Error(`Simulation error: ${result.error}`);
    }

    const returnVal = (result as SorobanRpc.Api.SimulateTransactionSuccessResponse)
      .result?.retval;

    if (!returnVal || returnVal.switch() === xdr.ScValType.scvVoid()) {
      return 0;
    }

    return scValToNative(returnVal) as number;
  }

  async transferTicket(
    params: TransferParams,
    publicKey: string,
    signFn: SignFunction,
  ): Promise<string> {
    const operation = this.contract.call(
      "transfer_ticket",
      nativeToScVal(params.eventId,       { type: "u32"     }),
      nativeToScVal(params.ticketId,      { type: "u32"     }),
      nativeToScVal(params.from,          { type: "address" }),
      nativeToScVal(params.to,            { type: "address" }),
      nativeToScVal(params.amount,        { type: "i128"    }),
      nativeToScVal(params.tokenAddress,  { type: "address" }),
    );

    return this._signAndSubmit(operation, publicKey, signFn);
  }

  async checkIn(
    eventId:    number,
    ticketId:   number,
    owner:      string,
    signFn:     SignFunction,
  ): Promise<string> {
    const operation = this.contract.call(
      "check_in",
      nativeToScVal(eventId,  { type: "u32"     }),
      nativeToScVal(ticketId, { type: "u32"     }),
      nativeToScVal(owner,    { type: "address" }),
    );

    return this._signAndSubmit(operation, owner, signFn);
  }

  private async _signAndSubmit(
    operation: xdr.Operation,
    publicKey: string,
    signFn:    SignFunction,
  ): Promise<string> {
    const account = await this.server.getAccount(publicKey).catch(() => null);
    if (!account) {
      throw new Error("Account not found on the network. Is it funded?");
    }

    let tx = new TransactionBuilder(account, {
      fee:               BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(operation)
      .setTimeout(30)
      .build();

    const simResult = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simResult)) {
      throw new Error(`Simulation failed: ${simResult.error}`);
    }

    const assembled = SorobanRpc.assembleTransaction(tx, simResult).build();
    const signedXdr = await signFn(assembled.toXDR());
    const signedTx  = new Transaction(signedXdr, this.networkPassphrase);

    const sendResult = await this.server.sendTransaction(signedTx);
    if (sendResult.status === "ERROR") {
      throw new Error(`Submission failed: ${JSON.stringify(sendResult.errorResult)}`);
    }

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