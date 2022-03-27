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
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const signer = createWallet(terra);

  const { txhash } = await sendTxWithConfirm(signer, [
    new MsgExecuteContract(signer.key.accAddress, argv["contract-address"], {
      finalize_arb: {},
    }),
  ]);
  console.log("Success! Txhash:", txhash);
})();
