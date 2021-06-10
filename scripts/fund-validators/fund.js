const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');

const endpoint = 'ws://localhost:9944';
const coinbaseUri = '//Alice';
const userAddress = {
  'bob': '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
  'charlie': '5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y',
};

async function main() {
  const wsProvider = new WsProvider(endpoint);
  const api = await ApiPromise.create({
    provider: wsProvider,
  });
  const keyring = new Keyring({ type: 'sr25519' });
  const coinbaseKeyring = keyring.addFromUri(coinbaseUri);

  const [chain, nodeName, nodeVersion] = await Promise.all([
    api.rpc.system.chain(),
    api.rpc.system.name(),
    api.rpc.system.version()
  ]);
  console.log(`You are connected to chain ${chain} using ${nodeName} v${nodeVersion}\n`);

  // Before tx, check balances
  console.log(`${coinbaseKeyring.address} has ${(await api.query.system.account(coinbaseKeyring.address)).data.free.toString()} (coinbase)`);
  for (let [name, address] of Object.entries(userAddress)) {
    let r = await api.query.system.account(address);
    console.log(`${address} has ${r.data.free.toString()} (${name})`);
  }

  // Send balance
  const balanceSend = 10 ** 11; // 10^11 pUnit = 0.1 Unit
  for (let [name, address] of Object.entries(userAddress)) {
    process.stdout.write(`Coinbase -> ${address} for ${balanceSend} pUnit (${name}) ... `);
    await new Promise(resolve => {
      api.tx.balances
        .transfer(address, balanceSend)
        .signAndSend(coinbaseKeyring, (result) => {
          if (result.status.isInBlock) {
            console.log(`Confirmed.`);
            resolve();
          }
        });
    });
  }

  // After tx, check balances
  console.log(`${coinbaseKeyring.address} has ${(await api.query.system.account(coinbaseKeyring.address)).data.free.toString()} (coinbase)`);
  for (let [name, address] of Object.entries(userAddress)) {
    let r = await api.query.system.account(address);
    console.log(`${address} has ${r.data.free.toString()} (${name})`);
  }
}

main().catch(console.error).finally(() => process.exit());
