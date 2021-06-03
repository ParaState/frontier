const help = `--evm-address <address>: Calculate the EVM address that corresponds to a native Substrate address.`;

module.exports = () => {
  if (process.argv.length < 4) {
    console.error('Please provide the <address> parameter.');
    console.error(help);
    process.exit(9);
  }

  const address = process.argv[3];
  if (!address.match(/^[A-z0-9]{48}$/)) {
    console.error('Please enter a valid Substrate address.');
    console.error(help);
    process.exit(9);
  }

  const { u8aToHex } = require("@polkadot/util");
  const { addressToEvm, evmToAddress } = require("@polkadot/util-crypto");

  return u8aToHex(addressToEvm(address));
};
