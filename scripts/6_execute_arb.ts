import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import { createLCDClient, createWallet, sendTxWithConfirm } from "./helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
    },
    "contract-address": {
      type: "string",
      demandOption: true,
    },
    amount: {
      type: "string",
      demandOption: true,
    },
    "minimum-profit": {
      type: "string",
      demandOption: false,
      default: "0.05",
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const signer = createWallet(terra);

  const { txhash } = await sendTxWithConfirm(signer, [
    new MsgExecuteContract(signer.key.accAddress, argv["contract-address"], {
      execute_arb: {
        amount: argv["amount"],
        minimum_profit: argv["minimum_profit"],
      },
    }),
  ]);
  console.log("Success! Txhash:", txhash);
})();
