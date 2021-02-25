const utils = require('./utils');

(async () => {
  const web3 = utils.web3();
  const accounts = await web3.eth.getAccounts();
  console.log('accounts: ', accounts);
  console.log('balance: ', await web3.eth.getBalance(accounts[0]));
  console.log('nonce: ', await web3.eth.getTransactionCount(accounts[0]));

  const r = await web3.eth.sendTransaction({
    from: accounts[0],
    to: accounts[1],
  });
  console.log("receipt: ", r);

  await utils.provider.engine.stop();
})();
