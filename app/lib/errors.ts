import {
  isSolanaError,
  SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM,
} from "@solana/kit";
import {
  getVestingErrorMessage,
  VESTING_ERROR__VESTING_ALREADY_EXISTS,
  VESTING_ERROR__INVALID_AMOUNT,
  type VestingError,
} from "../generated/vesting";

const VESTING_ERROR_CODES: Record<number, VestingError> = {
  [VESTING_ERROR__VESTING_ALREADY_EXISTS]: VESTING_ERROR__VESTING_ALREADY_EXISTS,
  [VESTING_ERROR__INVALID_AMOUNT]: VESTING_ERROR__INVALID_AMOUNT,
};

export function parseTransactionError(err: unknown): string {
  // Wallet rejection (comes from wallet-standard, not a SolanaError)
  if (err instanceof Error && err.message.includes("User rejected")) {
    return "Transaction was rejected by the wallet.";
  }

  // Anchor custom program errors — use the Codama-generated error messages
  if (
    isSolanaError(err, SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM) &&
    typeof err.context?.code === "number"
  ) {
    const vestingError = VESTING_ERROR_CODES[err.context.code];
    if (vestingError !== undefined) {
      return getVestingErrorMessage(vestingError);
    }
  }

  // For all other errors, kit's SolanaError already has readable messages.
  // Walk the cause chain to find the deepest message.
  const message = getDeepestMessage(err);
  return message.length > 200 ? `${message.slice(0, 200)}...` : message;
}

function getDeepestMessage(err: unknown): string {
  let deepest = err instanceof Error ? err.message : String(err);
  let current: unknown = err;

  while (current instanceof Error && current.cause) {
    current = current.cause;
    if (current instanceof Error) {
      deepest = current.message;
    }
  }

  return deepest;
}
