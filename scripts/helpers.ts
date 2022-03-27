import * as fs from "fs";
import * as promptly from "promptly";
import dotenv from "dotenv";
import {
  isTxError,
  LCDClient,
  LocalTerra,
  MnemonicKey,
  Msg,
  MsgInstantiateContract,
  MsgStoreCode,
  Wallet,
} from "@terra-money/terra.js";

const DEFAULT_GAS_SETTINGS = {
  gasPrices: "0.15uusd",
  gasAdjustment: 1.4,
};

/**
 * @notice Create an `LCDClient` instance based on provided network identifier
 */
export function createLCDClient(network: string): LCDClient {
  if (network === "mainnet") {
    return new LCDClient({
      chainID: "columbus-5",
      URL: "https://lcd.terra.dev",
    });
  } else if (network === "testnet") {
    return new LCDClient({
      chainID: "bombay-12",
      URL: "https://bombay-lcd.terra.dev",
    });
  } else if (network === "localterra") {
    return new LocalTerra();
  } else {
    throw new Error(`invalid network: ${network}, must be mainnet|testnet|localterra`);
  }
}

/**
 * @notice Create a `Wallet` instance by loading the mnemonic phrase stored in `.env`
 */
export function createWallet(terra: LCDClient): Wallet {
  dotenv.config();
  if (!process.env.MNEMONIC) {
    throw new Error("mnemonic not provided");
  }
  return terra.wallet(
    new MnemonicKey({
      mnemonic: process.env.MNEMONIC,
    })
  );
}

/**
 * @notice Returns contract addresses of the selected network
 */
export function getContractAddresses(network: string) {
  if (network === "mainnet") {
    return {
      mars_token: "terra12hgwnpupflfpuual532wgrxu2gjp0tcagzgx4n",
      mars_council: "terra1685de0sx5px80d47ec2xjln224phshysqxxeje",
      mars_red_bank: "terra19dtgj9j5j7kyf3pmejqv8vzfpxtejaypgzkz5u",
      astroport_mars_ust_pair: "terra19wauh79y42u5vt62c5adt2g5h4exgh26t3rpds",
    };
  } else if (network === "testnet") {
    return {
      mars_token: "terra1h9tmwpwll5zpx6dvu28t8mvjk9jctu9nftm5ru",
      mars_council: "terra1jtdz9fhrrwd8yak6e3z7utmkypvx0qf0n393c6",
      mars_red_bank: "terra1avkm5w0gzwm92h0dlxymsdhx4l2rm7k0lxnwq7",
      astroport_mars_ust_pair: "terra144m28x7d3lzjzp423mdydll6cmfafg407ve3ev",
    };
  } else {
    throw new Error(`invalid network: ${network}, must be mainnet|testnet`);
  }
}

/**
 * @notice Same with `sendTransaction`, but requires confirmation for CLI before broadcasting
 */
export async function sendTxWithConfirm(signer: Wallet, msgs: Msg[]) {
  const tx = await signer.createAndSignTx({ msgs, ...DEFAULT_GAS_SETTINGS });
  console.log("\n" + JSON.stringify(tx).replace(/\\/g, "") + "\n");

  const proceed = await promptly.confirm("Confirm transaction before broadcasting [y/N]:");
  if (!proceed) {
    console.log("User aborted!");
    process.exit(1);
  }

  const result = await signer.lcd.tx.broadcast(tx);
  if (isTxError(result)) {
    throw new Error(`tx failed! raw log: ${result.raw_log}`);
  }
  return result;
}

/**
 * @notice Same with `storeCode`, but requires confirmation for CLI before broadcasting
 */
export async function storeCodeWithConfirm(signer: Wallet, filePath: string) {
  const code = fs.readFileSync(filePath).toString("base64");
  const result = await sendTxWithConfirm(signer, [new MsgStoreCode(signer.key.accAddress, code)]);
  return parseInt(result.logs[0].eventsByType.store_code.code_id[0]);
}

/**
 * @notice Same with `instantiateContract`, but requires confirmation for CLI before broadcasting
 */
export async function instantiateWithConfirm(
  signer: Wallet,
  admin: string,
  codeId: number,
  initMsg: object
) {
  const result = await sendTxWithConfirm(signer, [
    new MsgInstantiateContract(signer.key.accAddress, admin, codeId, initMsg),
  ]);
  return result;
}

/**
 * Encode a JSON object to base64 string
 */
export function encodeBase64(obj: object | string | number) {
  return Buffer.from(JSON.stringify(obj)).toString("base64");
}
