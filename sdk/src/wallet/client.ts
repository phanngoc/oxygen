import {
  Connection,
  PublicKey,
  Transaction,
  SystemProgram,
  sendAndConfirmTransaction,
  TransactionInstruction,
  Signer,
  Keypair,
  LAMPORTS_PER_SOL,
  Commitment,
} from '@solana/web3.js';
import { WalletAdapter } from './index';
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import { BN } from 'bn.js';
import { Buffer } from 'buffer';

// Program ID for the Oxygen protocol
const OXYGEN_PROGRAM_ID = new PublicKey('Oxygen111111111111111111111111111111111111111');

/**
 * Client for interacting with the Oxygen protocol
 * Ensures all operations are non-custodial - users always sign their own transactions
 */
export class OxygenClient {
  private connection: Connection;
  private wallet: WalletAdapter;
  
  /**
   * Create a new Oxygen client
   * @param connection Solana connection
   * @param wallet Wallet adapter for signing transactions
   */
  constructor(connection: Connection, wallet: WalletAdapter) {
    this.connection = connection;
    this.wallet = wallet;
  }
  
  /**
   * Deposit tokens into the protocol
   * @param poolAddress Address of the pool to deposit into
   * @param tokenMint Mint of the token to deposit
   * @param amount Amount to deposit
   * @param useAsCollateral Whether to use the deposit as collateral
   * @param enableLending Whether to enable lending for this deposit
   * @returns Transaction signature
   */
  async deposit(
    poolAddress: PublicKey,
    tokenMint: PublicKey,
    amount: BN,
    useAsCollateral: boolean,
    enableLending: boolean,
  ): Promise<string> {
    if (!this.wallet.publicKey) {
      throw new Error('Wallet not connected');
    }
    
    // Get pool account data
    const poolAccountInfo = await this.connection.getAccountInfo(poolAddress);
    if (!poolAccountInfo) {
      throw new Error('Pool not found');
    }
    
    // Find user token account
    const userTokenAccount = await Token.getAssociatedTokenAddress(
      new PublicKey(TOKEN_PROGRAM_ID),
      new PublicKey(TOKEN_PROGRAM_ID),
      tokenMint,
      this.wallet.publicKey
    );
    
    // Find pool reserve account (PDA)
    const [assetReserve] = await PublicKey.findProgramAddress(
      [
        Buffer.from('reserve'),
        poolAddress.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Find user position account (PDA)
    const [userPosition] = await PublicKey.findProgramAddress(
      [
        Buffer.from('position'),
        this.wallet.publicKey.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Create the instruction data buffer
    const instructionData = Buffer.concat([
      Buffer.from(new Uint8Array([0])), // Deposit instruction discriminator
      Buffer.from(new Uint8Array(amount.toArray('le', 8))), // Amount as u64 LE
      Buffer.from(new Uint8Array([useAsCollateral ? 1 : 0])), // Use as collateral flag
      Buffer.from(new Uint8Array([enableLending ? 1 : 0])), // Enable lending flag
    ]);
    
    // Create the transaction instruction
    const depositInstruction = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: poolAddress, isSigner: false, isWritable: true },
        { pubkey: userTokenAccount, isSigner: false, isWritable: true },
        { pubkey: assetReserve, isSigner: false, isWritable: true },
        { pubkey: userPosition, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: OXYGEN_PROGRAM_ID,
      data: instructionData,
    });
    
    // Create and sign the transaction
    const transaction = new Transaction().add(depositInstruction);
    transaction.feePayer = this.wallet.publicKey;
    
    const { blockhash } = await this.connection.getRecentBlockhash();
    transaction.recentBlockhash = blockhash;
    
    // NON-CUSTODIAL: User signs the transaction with their wallet
    // The protocol never has access to user's private keys
    const signedTransaction = await this.wallet.signTransaction(transaction);
    
    // Send the signed transaction
    const signature = await this.connection.sendRawTransaction(signedTransaction.serialize());
    
    // Wait for confirmation
    await this.connection.confirmTransaction(signature);
    
    return signature;
  }
  
  /**
   * Withdraw tokens from the protocol
   * @param poolAddress Address of the pool to withdraw from
   * @param tokenMint Mint of the token to withdraw
   * @param amount Amount to withdraw
   * @param isLendingWithdrawal Whether this is a withdrawal from a lending position
   * @returns Transaction signature
   */
  async withdraw(
    poolAddress: PublicKey,
    tokenMint: PublicKey,
    amount: BN,
    isLendingWithdrawal: boolean,
  ): Promise<string> {
    if (!this.wallet.publicKey) {
      throw new Error('Wallet not connected');
    }
    
    // Find user token account
    const userTokenAccount = await Token.getAssociatedTokenAddress(
      new PublicKey(TOKEN_PROGRAM_ID),
      new PublicKey(TOKEN_PROGRAM_ID),
      tokenMint,
      this.wallet.publicKey
    );
    
    // Find pool reserve account (PDA)
    const [assetReserve] = await PublicKey.findProgramAddress(
      [
        Buffer.from('reserve'),
        poolAddress.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Find user position account (PDA)
    const [userPosition] = await PublicKey.findProgramAddress(
      [
        Buffer.from('position'),
        this.wallet.publicKey.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Create the instruction data buffer
    const instructionData = Buffer.concat([
      Buffer.from(new Uint8Array([1])), // Withdraw instruction discriminator
      Buffer.from(new Uint8Array(amount.toArray('le', 8))), // Amount as u64 LE
      Buffer.from(new Uint8Array([isLendingWithdrawal ? 1 : 0])), // Is lending withdrawal flag
    ]);
    
    // Create the transaction instruction
    const withdrawInstruction = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: poolAddress, isSigner: false, isWritable: true },
        { pubkey: userTokenAccount, isSigner: false, isWritable: true },
        { pubkey: assetReserve, isSigner: false, isWritable: true },
        { pubkey: userPosition, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      programId: OXYGEN_PROGRAM_ID,
      data: instructionData,
    });
    
    // Create and sign the transaction
    const transaction = new Transaction().add(withdrawInstruction);
    transaction.feePayer = this.wallet.publicKey;
    
    const { blockhash } = await this.connection.getRecentBlockhash();
    transaction.recentBlockhash = blockhash;
    
    // NON-CUSTODIAL: User signs the transaction with their wallet
    const signedTransaction = await this.wallet.signTransaction(transaction);
    
    // Send the signed transaction
    const signature = await this.connection.sendRawTransaction(signedTransaction.serialize());
    
    // Wait for confirmation
    await this.connection.confirmTransaction(signature);
    
    return signature;
  }
  
  /**
   * Borrow tokens from the protocol
   * @param poolAddress Address of the pool to borrow from
   * @param tokenMint Mint of the token to borrow
   * @param amount Amount to borrow
   * @param maintainCollateralLending Whether to maintain lending position
   * @returns Transaction signature
   */
  async borrow(
    poolAddress: PublicKey,
    tokenMint: PublicKey,
    amount: BN,
    maintainCollateralLending: boolean,
  ): Promise<string> {
    if (!this.wallet.publicKey) {
      throw new Error('Wallet not connected');
    }
    
    // Find user token account
    const userTokenAccount = await Token.getAssociatedTokenAddress(
      new PublicKey(TOKEN_PROGRAM_ID),
      new PublicKey(TOKEN_PROGRAM_ID),
      tokenMint,
      this.wallet.publicKey
    );
    
    // Find pool reserve account (PDA)
    const [assetReserve] = await PublicKey.findProgramAddress(
      [
        Buffer.from('reserve'),
        poolAddress.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Find user position account (PDA)
    const [userPosition] = await PublicKey.findProgramAddress(
      [
        Buffer.from('position'),
        this.wallet.publicKey.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Create the instruction data buffer
    const instructionData = Buffer.concat([
      Buffer.from(new Uint8Array([2])), // Borrow instruction discriminator
      Buffer.from(new Uint8Array(amount.toArray('le', 8))), // Amount as u64 LE
      Buffer.from(new Uint8Array([maintainCollateralLending ? 1 : 0])), // Maintain collateral lending flag
    ]);
    
    // Create the transaction instruction
    const borrowInstruction = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: poolAddress, isSigner: false, isWritable: true },
        { pubkey: userTokenAccount, isSigner: false, isWritable: true },
        { pubkey: assetReserve, isSigner: false, isWritable: true },
        { pubkey: userPosition, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: OXYGEN_PROGRAM_ID,
      data: instructionData,
    });
    
    // Create and sign the transaction
    const transaction = new Transaction().add(borrowInstruction);
    transaction.feePayer = this.wallet.publicKey;
    
    const { blockhash } = await this.connection.getRecentBlockhash();
    transaction.recentBlockhash = blockhash;
    
    // NON-CUSTODIAL: User signs the transaction with their wallet
    const signedTransaction = await this.wallet.signTransaction(transaction);
    
    // Send the signed transaction
    const signature = await this.connection.sendRawTransaction(signedTransaction.serialize());
    
    // Wait for confirmation
    await this.connection.confirmTransaction(signature);
    
    return signature;
  }
  
  /**
   * Repay borrowed tokens
   * @param poolAddress Address of the pool to repay to
   * @param tokenMint Mint of the token to repay
   * @param amount Amount to repay (use BN(-1) to repay full amount)
   * @returns Transaction signature
   */
  async repay(
    poolAddress: PublicKey,
    tokenMint: PublicKey,
    amount: BN,
  ): Promise<string> {
    if (!this.wallet.publicKey) {
      throw new Error('Wallet not connected');
    }
    
    // Find user token account
    const userTokenAccount = await Token.getAssociatedTokenAddress(
      new PublicKey(TOKEN_PROGRAM_ID),
      new PublicKey(TOKEN_PROGRAM_ID),
      tokenMint,
      this.wallet.publicKey
    );
    
    // Find pool reserve account (PDA)
    const [assetReserve] = await PublicKey.findProgramAddress(
      [
        Buffer.from('reserve'),
        poolAddress.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Find user position account (PDA)
    const [userPosition] = await PublicKey.findProgramAddress(
      [
        Buffer.from('position'),
        this.wallet.publicKey.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Create the instruction data buffer
    const instructionData = Buffer.concat([
      Buffer.from(new Uint8Array([3])), // Repay instruction discriminator
      Buffer.from(new Uint8Array(amount.toArray('le', 8))), // Amount as u64 LE
    ]);
    
    // Create the transaction instruction
    const repayInstruction = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: poolAddress, isSigner: false, isWritable: true },
        { pubkey: userTokenAccount, isSigner: false, isWritable: true },
        { pubkey: assetReserve, isSigner: false, isWritable: true },
        { pubkey: userPosition, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      programId: OXYGEN_PROGRAM_ID,
      data: instructionData,
    });
    
    // Create and sign the transaction
    const transaction = new Transaction().add(repayInstruction);
    transaction.feePayer = this.wallet.publicKey;
    
    const { blockhash } = await this.connection.getRecentBlockhash();
    transaction.recentBlockhash = blockhash;
    
    // NON-CUSTODIAL: User signs the transaction with their wallet
    const signedTransaction = await this.wallet.signTransaction(transaction);
    
    // Send the signed transaction
    const signature = await this.connection.sendRawTransaction(signedTransaction.serialize());
    
    // Wait for confirmation
    await this.connection.confirmTransaction(signature);
    
    return signature;
  }
  
  /**
   * Get user position information
   * @returns User position data
   */
  async getUserPosition(): Promise<any> {
    if (!this.wallet.publicKey) {
      throw new Error('Wallet not connected');
    }
    
    // Find user position account (PDA)
    const [userPosition] = await PublicKey.findProgramAddress(
      [
        Buffer.from('position'),
        this.wallet.publicKey.toBuffer(),
      ],
      OXYGEN_PROGRAM_ID
    );
    
    // Fetch the user position account data
    const accountInfo = await this.connection.getAccountInfo(userPosition);
    
    if (!accountInfo) {
      throw new Error('User position not found');
    }
    
    // In a real implementation, this would deserialize the account data
    // into a proper UserPosition structure
    
    return {
      positionAddress: userPosition.toString(),
      owner: this.wallet.publicKey.toString(),
      // More position data would be deserialized here
    };
  }
}