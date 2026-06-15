"use client";

import { useState, useEffect, useCallback } from "react";
import { useWallet } from "../lib/wallet/context";
import { useSendTransaction } from "../lib/hooks/use-send-transaction";
import { useBalance } from "../lib/hooks/use-balance";
import { lamportsToSolString } from "../lib/lamports";
import { type Address, address } from "@solana/kit";
import { toast } from "sonner";
import {
  getCreateVestingAccountInstructionAsync,
  getCreateEmployeeVestingInstructionAsync,
  getClaimTokensInstructionAsync,
} from "../generated/vesting/instructions";
import { findVestingAccountPda, findTreasuryTokenAccountPda, findEmployeeAccountPda } from "../generated/vesting/pdas";
import { fetchVestingAccount } from "../generated/vesting/accounts";
import { parseTransactionError } from "../lib/errors";
import { useCluster } from "./cluster-context";
import { useSolanaClient } from "../lib/solana-client-context";

export function VestingCard() {
  const { wallet, signer, status } = useWallet();
  const { send, isSending } = useSendTransaction();
  const { getExplorerUrl } = useCluster();
  const client = useSolanaClient();

  const [view, setView] = useState<"admin" | "employee">("employee");
  const walletAddress = wallet?.account.address;

  // Admin State
  const [companyName, setCompanyName] = useState("Acme Corp");
  const [mintAddress, setMintAddress] = useState("");
  const [employeeAddress, setEmployeeAddress] = useState("");
  const [amount, setAmount] = useState("");
  const [durationDays, setDurationDays] = useState("365");
  const [cliffDays, setCliffDays] = useState("30");

  // Employee State
  const [empCompanyName, setEmpCompanyName] = useState("Acme Corp");
  const [vestingPda, setVestingPda] = useState<Address | null>(null);
  const [employeePda, setEmployeePda] = useState<Address | null>(null);

  // PDAs
  useEffect(() => {
    async function loadPdas() {
      if (empCompanyName) {
        const [vPda] = await findVestingAccountPda({ companyName: empCompanyName });
        setVestingPda(vPda);
        
        if (walletAddress) {
          const [ePda] = await findEmployeeAccountPda({
            beneficiary: walletAddress,
            vestingAccount: vPda,
          });
          setEmployeePda(ePda);
        }
      }
    }
    loadPdas();
  }, [empCompanyName, walletAddress]);

  const handleCreateVesting = useCallback(async () => {
    if (!signer || !companyName || !mintAddress) return;

    try {
      const instruction = await getCreateVestingAccountInstructionAsync({
        signer,
        mint: address(mintAddress),
        companyName,
      });

      const signature = await send({ instructions: [instruction] });

      toast.success("Company Vesting Created!", {
        description: (
          <a
            href={getExplorerUrl(`/tx/${signature}`)}
            target="_blank"
            rel="noopener noreferrer"
            className="underline"
          >
            View transaction
          </a>
        ),
      });
    } catch (err) {
      console.error("Create Vesting failed:", err);
      toast.error(parseTransactionError(err));
    }
  }, [companyName, mintAddress, signer, send, getExplorerUrl]);

  const handleCreateEmployeeVesting = useCallback(async () => {
    if (!signer || !companyName || !employeeAddress || !amount) return;

    try {
      const startTime = Math.floor(Date.now() / 1000);
      const endTime = startTime + (parseInt(durationDays) * 24 * 60 * 60);
      const cliffTime = startTime + (parseInt(cliffDays) * 24 * 60 * 60);
      
      const [vPda] = await findVestingAccountPda({ companyName });

      const instruction = await getCreateEmployeeVestingInstructionAsync({
        owner: signer,
        beneficiary: address(employeeAddress),
        vestingAccount: vPda,
        startTime: BigInt(startTime),
        endTime: BigInt(endTime),
        totalAmount: BigInt(parseFloat(amount) * 1_000_000_000), // Assuming 9 decimals for now
        cliffTime: BigInt(cliffTime),
      });

      const signature = await send({ instructions: [instruction] });

      toast.success("Employee Vesting Schedule Created!", {
        description: (
          <a
            href={getExplorerUrl(`/tx/${signature}`)}
            target="_blank"
            rel="noopener noreferrer"
            className="underline"
          >
            View transaction
          </a>
        ),
      });
    } catch (err) {
      console.error("Create Employee failed:", err);
      toast.error(parseTransactionError(err));
    }
  }, [companyName, employeeAddress, amount, durationDays, cliffDays, signer, send, getExplorerUrl]);

  const handleClaimTokens = useCallback(async () => {
    if (!signer || !empCompanyName || !vestingPda || !employeePda) return;

    try {
      const vAccount = await fetchVestingAccount(client.rpc, vestingPda);
      const mint = vAccount.data.mint;
      const treasuryTokenAccount = vAccount.data.treasuryTokenAccount;

      const instruction = await getClaimTokensInstructionAsync({
        beneficiary: signer,
        employeeAccount: employeePda,
        vestingAccount: vestingPda,
        mint,
        treasuryTokenAccount,
        companyName: empCompanyName,
      });

      const signature = await send({ instructions: [instruction] });

      toast.success("Tokens Claimed Successfully!", {
        description: (
          <a
            href={getExplorerUrl(`/tx/${signature}`)}
            target="_blank"
            rel="noopener noreferrer"
            className="underline"
          >
            View transaction
          </a>
        ),
      });
    } catch (err) {
      console.error("Claim failed:", err);
      toast.error(parseTransactionError(err));
    }
  }, [signer, empCompanyName, vestingPda, employeePda, send, getExplorerUrl]);

  if (status !== "connected") {
    return (
      <section className="glass-card w-full space-y-4 rounded-2xl p-8">
        <div className="space-y-2 text-center">
          <p className="text-2xl font-bold glow-text">Connect Wallet</p>
          <p className="text-sm text-muted-foreground">
            Connect your wallet to access the Vesting Portal.
          </p>
        </div>
      </section>
    );
  }

  return (
    <section className="glass-card w-full overflow-hidden rounded-2xl border border-primary/20 shadow-[0_0_50px_-12px_rgba(157,78,221,0.2)]">
      {/* Tabs */}
      <div className="flex border-b border-primary/10">
        <button
          onClick={() => setView("employee")}
          className={`flex-1 py-4 text-sm font-medium transition ${
            view === "employee" ? "bg-primary/10 text-primary glow-text border-b-2 border-primary" : "text-muted-foreground hover:bg-white/5"
          }`}
        >
          Employee Portal
        </button>
        <button
          onClick={() => setView("admin")}
          className={`flex-1 py-4 text-sm font-medium transition ${
            view === "admin" ? "bg-primary/10 text-primary glow-text border-b-2 border-primary" : "text-muted-foreground hover:bg-white/5"
          }`}
        >
          Company Admin
        </button>
      </div>

      <div className="p-6 space-y-6">
        {view === "admin" && (
          <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {/* Initialize Company */}
            <div className="space-y-4 rounded-xl border border-white/5 bg-black/20 p-5">
              <div>
                <h3 className="text-lg font-semibold text-foreground">1. Initialize Company Vesting</h3>
                <p className="text-xs text-muted-foreground mt-1">Create the main vesting and treasury account for your token.</p>
              </div>
              <div className="space-y-3">
                <input
                  type="text"
                  placeholder="Company Name"
                  value={companyName}
                  onChange={(e) => setCompanyName(e.target.value)}
                  className="w-full rounded-lg border border-border-low bg-black/40 px-4 py-2.5 text-sm outline-none transition focus:border-primary/50 focus:ring-1 focus:ring-primary"
                />
                <input
                  type="text"
                  placeholder="Token Mint Address"
                  value={mintAddress}
                  onChange={(e) => setMintAddress(e.target.value)}
                  className="w-full rounded-lg border border-border-low bg-black/40 px-4 py-2.5 text-sm outline-none transition focus:border-primary/50 focus:ring-1 focus:ring-primary"
                />
                <button
                  onClick={handleCreateVesting}
                  disabled={isSending || !companyName || !mintAddress}
                  className="w-full rounded-lg bg-primary/20 border border-primary/30 px-5 py-2.5 text-sm font-medium text-primary shadow-xs transition hover:bg-primary/30 disabled:opacity-50"
                >
                  {isSending ? "Processing..." : "Create Vesting Vault"}
                </button>
              </div>
            </div>

            {/* Create Employee Schedule */}
            <div className="space-y-4 rounded-xl border border-white/5 bg-black/20 p-5">
              <div>
                <h3 className="text-lg font-semibold text-foreground">2. Add Employee Schedule</h3>
                <p className="text-xs text-muted-foreground mt-1">Issue locked tokens to an employee with a custom timeline.</p>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <input
                  type="text"
                  placeholder="Employee Wallet Address"
                  value={employeeAddress}
                  onChange={(e) => setEmployeeAddress(e.target.value)}
                  className="col-span-2 w-full rounded-lg border border-border-low bg-black/40 px-4 py-2.5 text-sm outline-none transition focus:border-secondary/50 focus:ring-1 focus:ring-secondary"
                />
                <input
                  type="number"
                  placeholder="Total Tokens"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  className="col-span-2 w-full rounded-lg border border-border-low bg-black/40 px-4 py-2.5 text-sm outline-none transition focus:border-secondary/50 focus:ring-1 focus:ring-secondary"
                />
                <input
                  type="number"
                  placeholder="Duration (Days)"
                  value={durationDays}
                  onChange={(e) => setDurationDays(e.target.value)}
                  className="w-full rounded-lg border border-border-low bg-black/40 px-4 py-2.5 text-sm outline-none transition focus:border-secondary/50 focus:ring-1 focus:ring-secondary"
                />
                <input
                  type="number"
                  placeholder="Cliff (Days)"
                  value={cliffDays}
                  onChange={(e) => setCliffDays(e.target.value)}
                  className="w-full rounded-lg border border-border-low bg-black/40 px-4 py-2.5 text-sm outline-none transition focus:border-secondary/50 focus:ring-1 focus:ring-secondary"
                />
                <button
                  onClick={handleCreateEmployeeVesting}
                  disabled={isSending || !employeeAddress || !amount || !companyName}
                  className="col-span-2 mt-2 w-full rounded-lg bg-secondary/20 border border-secondary/30 px-5 py-2.5 text-sm font-medium text-secondary transition hover:bg-secondary/30 disabled:opacity-50"
                >
                  {isSending ? "Processing..." : "Create Schedule"}
                </button>
              </div>
            </div>
          </div>
        )}

        {view === "employee" && (
          <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <div className="space-y-4">
              <label className="text-sm font-medium text-muted-foreground">Select Company</label>
              <input
                type="text"
                placeholder="Enter Company Name"
                value={empCompanyName}
                onChange={(e) => setEmpCompanyName(e.target.value)}
                className="w-full rounded-lg border border-border-low bg-black/40 px-4 py-2.5 text-sm outline-none transition focus:border-primary/50 focus:ring-1 focus:ring-primary"
              />
            </div>

            <div className="rounded-xl border border-white/5 bg-black/20 p-6 flex flex-col items-center text-center space-y-4 relative overflow-hidden">
              <div className="absolute top-0 right-0 p-4 opacity-10">
                <svg viewBox="0 0 24 24" fill="currentColor" className="w-24 h-24 text-primary"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/></svg>
              </div>
              <p className="text-sm uppercase tracking-wider text-muted-foreground z-10">Your Claimable Tokens</p>
              <h2 className="text-5xl font-black text-white glow-text tabular-nums z-10">
                {/* Normally we'd fetch the exact claimable amount via RPC, showing "Ready" here as UI placeholder */}
                Ready
              </h2>
              
              <button
                onClick={handleClaimTokens}
                disabled={isSending || !empCompanyName || !vestingPda}
                className="mt-6 w-full max-w-xs rounded-full bg-primary px-8 py-3.5 text-sm font-bold text-white shadow-[0_0_30px_-5px_rgba(157,78,221,0.6)] transition hover:scale-105 hover:bg-primary/90 disabled:opacity-50 disabled:hover:scale-100 z-10"
              >
                {isSending ? "Claiming..." : "Claim Vested Tokens"}
              </button>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
