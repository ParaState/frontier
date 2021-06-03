const help = `--evm-account <AccountId32> | <H160>: Calculate the EVM gas charge account that corresponds to a native Substrate Account or EVM address.`;

module.exports = () => {
  if (process.argv.length < 4) {
    console.error('Please provide the <account> or <address> parameter.');
    console.error(help);
    process.exit(9);
  }

  const address = process.argv[3];
  if (!address.match(/^[A-z0-9]{40}|0x[A-z0-9]{40}|[A-z0-9]{48}$/)) {
    console.error('Please enter a valid Substrate or Eth  address.');
    console.error(help);
    process.exit(9);
  }

  const { u8aToHex } = require("@polkadot/util");
  const { addressToEvm, evmToAddress } = require("@polkadot/util-crypto");

  var eth_addr;
  if (address.length == 48) {
    eth_addr = u8aToHex(addressToEvm(address));
  }
  else if (address.length == 42) {
    eth_addr = address;
  }
  else {
    eth_addr = '0x' + address;
  }
  return evmToAddress(eth_addr);
};
