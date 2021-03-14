const utils = require('./utils');

(async () => {
  const web3 = utils.web3();
  let accounts = await web3.eth.getAccounts();
  console.log(`accounts: ${JSON.stringify(accounts)}`);
  console.log(`balance: ${await web3.eth.getBalance(accounts[0])}`);

  for (const vm_type in utils.erc20bytecdoe) {
    let receipt;
    receipt = await web3.eth.sendTransaction({ from: accounts[0], data: utils.erc20bytecdoe[vm_type] });
    let contract = new web3.eth.Contract(utils.erc20abi, receipt.contractAddress);
    console.log(`ERC-20 ${vm_type} contract created at ${receipt.contractAddress}`);

    // check balances
    let result;
    result = await contract.methods.balanceOf(accounts[0]).call();
    console.log(`contract.balanceOf(${accounts[0]}) = ${result}`);
    result = await contract.methods.balanceOf(accounts[1]).call();
    console.log(`contract.balanceOf(${accounts[1]}) = ${result}`);

    let amount = 1;
    await contract.methods.transfer(accounts[1], amount).send({ from: accounts[0] }).on('receipt', function (receipt) {
      console.log(`Transfer ${amount} token from address(${accounts[0]}) to address(${accounts[1]})`);
    });

    // check balances
    result = await contract.methods.balanceOf(accounts[0]).call();
    console.log(`contract.balanceOf(${accounts[0]}) = ${result}`);
    result = await contract.methods.balanceOf(accounts[1]).call();
    console.log(`contract.balanceOf(${accounts[1]}) = ${result}`);
  }

  await utils.provider.engine.stop();
})();
