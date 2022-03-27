import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import { createLCDClient, createWallet, getContractAddresses, sendTxWithConfirm } from "./helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
    },
    "proposal-id": {
      type: "number",
      demandOption: true,
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const signer = createWallet(terra);
  const contracts = getContractAddresses(argv["network"]);

  const { txhash } = await sendTxWithConfirm(signer, [
    new MsgExecuteContract(signer.key.accAddress, contracts["mars_council"], {
      end_proposal: {
        proposal_id: argv["proposal-id"],
      },
    }),
  ]);
  console.log("Success! Txhash:", txhash);
})();
