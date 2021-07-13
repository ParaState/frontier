const utils = require('./utils');

(async () => {
  const web3 = utils.web3();
  let accounts = [
    '0x6be02d1d3665660d22ff9624b7be0551ee1ac91b',
    '0x1cCA28600d7491365520B31b466f88647B9839eC',
  ];

  while (true) {
    console.log('block: ', await web3.eth.getBlockNumber());
    for (account of accounts) {
      console.log('  accounts: ', account);
      console.log('  balance: ', await web3.eth.getBalance(account));
    }
    await new Promise(r => setTimeout(r, 6000));
  }

  await utils.provider.engine.stop();
})();
