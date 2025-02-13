import pkg from "@safe-global/protocol-kit";
import apiKitPkg from "@safe-global/api-kit";
import { Command } from 'commander';
const { default: SafeApiKit } = apiKitPkg;
const {
  default: Safe,
  SigningMethod,
  buildContractSignature,
  hashSafeMessage,
  buildSignatureBytes,
} = pkg;
import { sepolia } from "viem/chains";

const SEPOLIA_ID = 11155111;
const SEPOLIA_RPC = sepolia.rpcUrls.default.http[0];

async function signWithSafe(safeAddress, ownerKeys, message) {
  console.error("SEPOLIA URL: ", SEPOLIA_RPC);
  try {
    console.error("\n=== Starting Safe signing process ===");
    console.error("Safe Address:", safeAddress);
    console.error(
      "Owner Keys:",
      ownerKeys.map((k) => k.slice(0, 10) + "...")
    );

    // Initialize Protocol Kit
    console.error("\n1. Initializing Protocol Kit with first owner key...");
    let protocolKit = await Safe.init({
      provider: SEPOLIA_RPC,
      safeAddress: safeAddress,
      signer: ownerKeys[0],
    });
    console.error("Protocol Kit initialized");

    // Initialize API Kit
    console.error("\n2. Initializing API Kit...");
    const apiKit = new SafeApiKit({ chainId: SEPOLIA_ID });
    console.error("API Kit initialized");

    console.error("\n3. Creating message object...");
    let safeMessage = protocolKit.createMessage(message);
    console.error("Message created:", safeMessage);
    let signatures = [];
    let isFirstSigner = true;

    for (const ownerKey of ownerKeys) {
      console.error("\n4. Processing owner key:", ownerKey.slice(0, 10) + "...");

      console.error("Connecting owner to Safe...");
      protocolKit = await protocolKit.connect({
        provider: SEPOLIA_RPC,
        signer: ownerKey,
        safeAddress: safeAddress,
      });
      console.error("Owner connected");

      // Sign the message
      console.error("\n5. Signing message...");
      safeMessage = await protocolKit.signMessage(
        safeMessage,
        SigningMethod.ETH_SIGN_TYPED_DATA_V4
      );
      console.error("Message signed");
      console.error("Current signatures:", safeMessage.signatures);

      console.error("\n6. Building contract signature...");
      const contractSignature = await buildContractSignature(
        Array.from(safeMessage.signatures.values()),
        safeAddress
      );
      console.error("Contract signature built:", contractSignature);

      safeMessage.addSignature(contractSignature);
      console.error("Signature added to message");

      // Get the safe message hash
      const msgHash = hashSafeMessage(message);
      console.error("First message hash:", msgHash);

      const safeMessageHash = await protocolKit.getSafeMessageHash(msgHash);
      console.error("Message hash:", safeMessageHash);

      // First signer proposes the message
      if (isFirstSigner) {
        console.error("\n7a. Proposing message to Safe Transaction Service...");
        const ownerSignature = Array.from(safeMessage.signatures.values())[0];
        console.error("Using owner signature:", ownerSignature);

        await apiKit.addMessage(safeAddress, {
          message: message,
          signature: buildSignatureBytes([ownerSignature]),
        });
        console.error("Message proposed");
        isFirstSigner = false;
      } else {
        // Additional signers confirm the message
        console.error("\n7b. Adding confirmation signature...");
        await apiKit.addMessageSignature(
          safeMessageHash,
          buildSignatureBytes([contractSignature])
        );
        console.error("Confirmation added");
      }

      signatures.push({
        signer: await protocolKit.getAddress(),
        data: contractSignature.data,
      });
    }

    // Get the final message status
    const safeMessageHash = await protocolKit.getSafeMessageHash(
      hashSafeMessage(message)
    );
    console.error("\n8. Getting final message status...");
    const confirmedMessage = await apiKit.getMessage(safeMessageHash);
    console.error("Confirmed message:", confirmedMessage);

    return {
      success: true,
      message: safeMessage.data,
      signatures: signatures,
      messageHash: safeMessageHash,
      confirmedMessage,
    };
  } catch (error) {
    console.error("\n!!! Error in signWithSafe !!!");
    console.error("Error occurred:", error.message);
    console.error("Stack trace:", error.stack);
    return {
      success: false,
      error: error.message,
    };
  }
}

// CLI setup
const program = new Command();

program
  .name('safe-sign')
  .description('Sign a message with a Safe multisig')
  .argument('<safe-address>', 'address of the Safe contract')
  .argument('<owner-keys>', 'comma-separated list of owner private keys')
  .argument('<message>', 'message to sign')
  .action(async (safeAddress, ownerKeysStr, message) => {
    const ownerKeys = ownerKeysStr.split(',');
    try {
      const result = await signWithSafe(safeAddress, ownerKeys, message);
      if (result.success) {
      } else {
        console.error("\nError:", result.error);
        process.exit(1);
      }
    } catch (error) {
      console.error("\nUnexpected error:", error);
      process.exit(1);
    }
  });

program.parse();
