import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import {
  createLCDClient,
  createWallet,
  getContractAddresses,
  encodeBase64,
  sendTxWithConfirm,
} from "./helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
    },
    account: {
      type: "string",
      demandOption: true,
    },
    amount: {
      type: "string",
      demandOption: true,
    },
    "deposit-amount": {
      type: "string",
      demandOption: false,
      default: "100000000000",
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const signer = createWallet(terra);
  const contracts = getContractAddresses(argv["network"]);

  // Query the current proposal count, so that we know the proposal ID we're about to create
  const proposalsResponse: { proposal_count: number } = await terra.wasm.contractQuery(
    contracts["mars_council"],
    {
      proposals: {},
    }
  );

  const { txhash } = await sendTxWithConfirm(signer, [
    new MsgExecuteContract(signer.key.accAddress, contracts["mars_token"], {
      send: {
        contract: contracts["mars_council"],
        amount: argv["deposit-amount"],
        msg: encodeBase64({
          submit_proposal: {
            title: "Update C2C credit limit",
            description: "Give Luna uncollateralized credit limit to leveraged arbitrage contract",
            link: undefined,
            messages: [
              {
                execution_order: 0,
                msg: {
                  wasm: {
                    execute: {
                      contract_addr: contracts["mars_red_bank"],
                      msg: encodeBase64({
                        update_uncollateralized_loan_limit: {
                          user_address: argv["account"],
                          asset: {
                            native: {
                              denom: "uluna",
                            },
                          },
                          new_limit: argv["amount"].toString(),
                        },
                      }),
                      funds: [],
                    },
                  },
                },
              },
            ],
          },
        }),
      },
    }),
    new MsgExecuteContract(signer.key.accAddress, contracts["mars_council"], {
      cast_vote: {
        proposal_id: proposalsResponse["proposal_count"] + 1,
        vote: "for",
      },
    }),
  ]);
  console.log("Success! Txhash:", txhash);
})();
