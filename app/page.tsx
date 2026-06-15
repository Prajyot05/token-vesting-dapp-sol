"use client";

import { useState } from "react";
import { lamports as sol } from "@solana/kit";
import { toast } from "sonner";
import { useWallet } from "./lib/wallet/context";
import { useBalance } from "./lib/hooks/use-balance";
import { lamportsToSolString } from "./lib/lamports";
import { useSolanaClient } from "./lib/solana-client-context";
import { ellipsify } from "./lib/explorer";
import { VestingCard } from "./components/vesting-card";
import { ThemeToggle } from "./components/theme-toggle";
import { ClusterSelect } from "./components/cluster-select";
import { WalletButton } from "./components/wallet-button";
import { useCluster } from "./components/cluster-context";

export default function Home() {
  const { wallet, status } = useWallet();
  const { cluster, getExplorerUrl } = useCluster();
  const client = useSolanaClient();

  const address = wallet?.account.address;
  const balance = useBalance(address);
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    if (!address) return;
    await navigator.clipboard.writeText(address);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleAirdrop = async () => {
    if (!address) return;
    try {
      toast.info("Requesting airdrop...");
      const sig = await client.airdrop(address, sol(1_000_000_000n));
      toast.success("Airdrop received!", {
        description: sig ? (
          <a
            href={getExplorerUrl(`/tx/${sig}`)}
            target="_blank"
            rel="noopener noreferrer"
            className="underline"
          >
            View transaction
          </a>
        ) : undefined,
      });
    } catch (err) {
      console.error("Airdrop failed:", err);
      const msg = err instanceof Error ? err.message : String(err);
      const isRateLimited =
        msg.includes("429") || msg.includes("Internal JSON-RPC error");
      toast.error(
        isRateLimited
          ? "Devnet faucet rate-limited. Use the web faucet instead."
          : "Airdrop failed. Try again later.",
        isRateLimited
          ? {
              description: (
                <a
                  href="https://faucet.solana.com/"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="underline"
                >
                  Open faucet.solana.com
                </a>
              ),
            }
          : undefined
      );
    }
  };

  return (
    <div className="relative min-h-screen bg-background text-foreground overflow-hidden">
      {/* Dynamic Background */}
      <div className="absolute inset-0 z-0">
        <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] rounded-full bg-primary/20 blur-[120px]" />
        <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] rounded-full bg-secondary/20 blur-[120px]" />
      </div>

      <div className="relative z-10">
        {/* Header */}
        <header className="mx-auto flex max-w-6xl items-center justify-between px-6 py-6 glass sticky top-0 z-50">
          <span className="text-xl font-bold tracking-tight glow-text text-primary">
            Nexus Vesting
          </span>
          <div className="flex items-center gap-4">
            <ThemeToggle />
            <ClusterSelect />
            <WalletButton />
          </div>
        </header>

        <main className="mx-auto max-w-6xl px-6">
          {/* Hero */}
          <section className="pt-12 pb-20 md:pt-20 md:pb-32 text-center flex flex-col items-center">
            <h1 className="font-black tracking-tight text-foreground max-w-4xl">
              <span className="block text-5xl md:text-7xl">Enterprise-Grade</span>
              <span className="block text-6xl md:text-8xl mt-2 bg-clip-text text-transparent bg-gradient-to-r from-primary to-secondary">Token Vesting</span>
            </h1>
            <p className="mt-6 text-lg leading-relaxed text-muted-foreground max-w-2xl">
              Securely lock and distribute SPL Tokens. Designed for founders to issue tokens to employees with configurable cliffs and durations. Built on Anchor and Solana.
            </p>
          </section>

          {/* Main Dashboard Content */}
          <div className="space-y-10 pb-20 max-w-4xl mx-auto">
            {/* Wallet Balance */}
            {status === "connected" && address && (
              <section className="relative w-full overflow-hidden rounded-2xl glass-card px-6 py-6 flex items-center justify-between border border-white/10">
                <div className="flex items-center gap-4">
                  <div className="flex h-12 w-12 items-center justify-center rounded-full bg-primary/20 border border-primary/30">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      className="h-5 w-5 text-primary"
                    >
                      <path d="M21 12V7H5a2 2 0 0 1 0-4h14v4" />
                      <path d="M3 5v14a2 2 0 0 0 2 2h16v-5" />
                      <path d="M18 12a2 2 0 0 0 0 4h4v-4Z" />
                    </svg>
                  </div>
                  <div>
                    <span className="text-sm font-medium text-muted-foreground block">Connected Wallet</span>
                    <button
                      onClick={handleCopy}
                      className="flex cursor-pointer items-center gap-2 font-mono text-lg font-semibold text-white transition hover:text-primary"
                    >
                      {ellipsify(address, 6)}
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        className="h-4 w-4"
                      >
                        {copied ? (
                          <path d="M20 6 9 17l-5-5" />
                        ) : (
                          <>
                            <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
                            <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
                          </>
                        )}
                      </svg>
                    </button>
                  </div>
                </div>
                
                <div className="text-right flex flex-col items-end gap-2">
                  <p className="font-mono text-3xl font-bold tabular-nums tracking-tight text-white">
                    {balance.lamports != null
                      ? lamportsToSolString(balance.lamports)
                      : "\u2014"}
                    <span className="ml-2 text-sm font-normal text-muted-foreground">
                      SOL
                    </span>
                  </p>
                  {cluster !== "mainnet" && (
                    <button
                      onClick={handleAirdrop}
                      className="cursor-pointer rounded-full bg-white/10 px-4 py-1.5 text-xs font-medium text-white transition hover:bg-white/20"
                    >
                      Airdrop Devnet SOL
                    </button>
                  )}
                </div>
              </section>
            )}

            {/* Vesting Program Section */}
            <VestingCard />
          </div>
        </main>
      </div>
    </div>
  );
}
