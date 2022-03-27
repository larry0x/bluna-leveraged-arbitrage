import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import { createLCDClient, createWallet, getContractAddresses, sendTxWithConfirm } from "./helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
    },
    "offer-amount": {
      type: "string",
      demandOption: true,
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const signer = createWallet(terra);
  const contracts = getContractAddresses(argv["network"]);

  const { txhash } = await sendTxWithConfirm(signer, [
    new MsgExecuteContract(
      signer.key.accAddress,
      contracts["astroport_mars_ust_pair"],
      {
        swap: {
          offer_asset: {
            info: {
              native_token: {
                denom: "uusd",
              },
            },
            amount: argv["offer-amount"],
          },
          max_spread: "0.5",
        },
      },
      { uusd: argv["offer-amount"] }
    ),
  ]);
  console.log("Success! Txhash:", txhash);
})();
