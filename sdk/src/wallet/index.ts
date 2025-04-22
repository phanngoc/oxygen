import {
  Connection,
  PublicKey,
  Transaction,
  SystemProgram,
  sendAndConfirmTransaction,
  TransactionInstruction,
  Signer,
} from '@solana/web3.js';
import { BN } from 'bn.js';

/**
 * Interface for any wallet adapter that connects to the Oxygen protocol
 * This ensures all wallets implement the required methods for non-custodial operation
 */
export interface WalletAdapter {
  /**
   * Public key of the connected wallet
   */
  publicKey: PublicKey | null;
  
  /**
   * Connect to the wallet
   */
  connect(): Promise<void>;
  
  /**
   * Disconnect from the wallet
   */
  disconnect(): Promise<void>;
  
  /**
   * Sign a transaction - this is the key to non-custodial operation
   * as only the user can sign transactions with their private key
   */
  signTransaction(transaction: Transaction): Promise<Transaction>;
  
  /**
   * Sign multiple transactions
   */
  signAllTransactions(transactions: Transaction[]): Promise<Transaction[]>;
  
  /**
   * Sign a message
   */
  signMessage(message: Uint8Array): Promise<Uint8Array>;
  
  /**
   * True if the wallet is connected
   */
  connected: boolean;
  
  /**
   * Name of the wallet (e.g. "Phantom" or "Solflare")
   */
  walletName: string;
}

/**
 * Adapter for Phantom wallet
 */
export class PhantomWalletAdapter implements WalletAdapter {
  private _publicKey: PublicKey | null = null;
  private _connected: boolean = false;
  
  constructor() {
    this.checkForPhantom();
  }
  
  /**
   * Check if Phantom wallet is available in the browser
   */
  private checkForPhantom() {
    if (typeof window !== 'undefined' && window.solana && window.solana.isPhantom) {
      console.log('Phantom wallet found!');
    } else {
      console.warn('Phantom wallet not found. Please install Phantom: https://phantom.app/');
    }
  }
  
  get publicKey(): PublicKey | null {
    return this._publicKey;
  }
  
  get connected(): boolean {
    return this._connected;
  }
  
  get walletName(): string {
    return 'Phantom';
  }
  
  async connect(): Promise<void> {
    try {
      if (typeof window === 'undefined' || !window.solana || !window.solana.isPhantom) {
        throw new Error('Phantom wallet not found. Please install Phantom wallet.');
      }
      
      const response = await window.solana.connect();
      this._publicKey = new PublicKey(response.publicKey.toString());
      this._connected = true;
      
      // Setup disconnect event handler
      window.solana.on('disconnect', () => {
        this._publicKey = null;
        this._connected = false;
      });
      
      console.log('Connected to Phantom wallet:', this._publicKey.toString());
    } catch (error) {
      console.error('Error connecting to Phantom wallet:', error);
      throw error;
    }
  }
  
  async disconnect(): Promise<void> {
    if (typeof window !== 'undefined' && window.solana) {
      await window.solana.disconnect();
      this._publicKey = null;
      this._connected = false;
    }
  }
  
  async signTransaction(transaction: Transaction): Promise<Transaction> {
    if (!this.connected || !window.solana) {
      throw new Error('Wallet not connected');
    }
    
    try {
      return await window.solana.signTransaction(transaction);
    } catch (error) {
      console.error('Error signing transaction:', error);
      throw error;
    }
  }
  
  async signAllTransactions(transactions: Transaction[]): Promise<Transaction[]> {
    if (!this.connected || !window.solana) {
      throw new Error('Wallet not connected');
    }
    
    try {
      return await window.solana.signAllTransactions(transactions);
    } catch (error) {
      console.error('Error signing transactions:', error);
      throw error;
    }
  }
  
  async signMessage(message: Uint8Array): Promise<Uint8Array> {
    if (!this.connected || !window.solana) {
      throw new Error('Wallet not connected');
    }
    
    try {
      // Sign the message and get the signature
      const { signature } = await window.solana.signMessage(message, 'utf8');
      return Uint8Array.from(signature);
    } catch (error) {
      console.error('Error signing message:', error);
      throw error;
    }
  }
}

/**
 * Adapter for Solflare wallet
 */
export class SolflareWalletAdapter implements WalletAdapter {
  private _publicKey: PublicKey | null = null;
  private _connected: boolean = false;
  
  constructor() {
    this.checkForSolflare();
  }
  
  /**
   * Check if Solflare wallet is available in the browser
   */
  private checkForSolflare() {
    if (typeof window !== 'undefined' && window.solflare) {
      console.log('Solflare wallet found!');
    } else {
      console.warn('Solflare wallet not found. Please install Solflare: https://solflare.com/');
    }
  }
  
  get publicKey(): PublicKey | null {
    return this._publicKey;
  }
  
  get connected(): boolean {
    return this._connected;
  }
  
  get walletName(): string {
    return 'Solflare';
  }
  
  async connect(): Promise<void> {
    try {
      if (typeof window === 'undefined' || !window.solflare) {
        throw new Error('Solflare wallet not found. Please install Solflare wallet.');
      }
      
      await window.solflare.connect();
      
      if (window.solflare.publicKey) {
        this._publicKey = new PublicKey(window.solflare.publicKey.toString());
        this._connected = true;
        
        // Setup disconnect event handler
        window.solflare.on('disconnect', () => {
          this._publicKey = null;
          this._connected = false;
        });
        
        console.log('Connected to Solflare wallet:', this._publicKey.toString());
      }
    } catch (error) {
      console.error('Error connecting to Solflare wallet:', error);
      throw error;
    }
  }
  
  async disconnect(): Promise<void> {
    if (typeof window !== 'undefined' && window.solflare) {
      await window.solflare.disconnect();
      this._publicKey = null;
      this._connected = false;
    }
  }
  
  async signTransaction(transaction: Transaction): Promise<Transaction> {
    if (!this.connected || !window.solflare) {
      throw new Error('Wallet not connected');
    }
    
    try {
      return await window.solflare.signTransaction(transaction);
    } catch (error) {
      console.error('Error signing transaction:', error);
      throw error;
    }
  }
  
  async signAllTransactions(transactions: Transaction[]): Promise<Transaction[]> {
    if (!this.connected || !window.solflare) {
      throw new Error('Wallet not connected');
    }
    
    try {
      return await window.solflare.signAllTransactions(transactions);
    } catch (error) {
      console.error('Error signing transactions:', error);
      throw error;
    }
  }
  
  async signMessage(message: Uint8Array): Promise<Uint8Array> {
    if (!this.connected || !window.solflare) {
      throw new Error('Wallet not connected');
    }
    
    try {
      // Sign the message and get the signature
      const { signature } = await window.solflare.signMessage(message, 'utf8');
      return Uint8Array.from(signature);
    } catch (error) {
      console.error('Error signing message:', error);
      throw error;
    }
  }
}

/**
 * Factory function to get appropriate wallet adapter based on the user's browser environment
 */
export function getWalletAdapter(): WalletAdapter {
  if (typeof window === 'undefined') {
    throw new Error('Cannot determine wallet type in non-browser environment');
  }
  
  // Check for Phantom first (most popular)
  if (window.solana && window.solana.isPhantom) {
    return new PhantomWalletAdapter();
  }
  
  // Check for Solflare
  if (window.solflare) {
    return new SolflareWalletAdapter();
  }
  
  // If no supported wallet is found, default to Phantom with a warning
  console.warn('No supported wallet found. Please install Phantom or Solflare.');
  return new PhantomWalletAdapter();
}