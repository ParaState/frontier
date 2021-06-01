import { expect } from "chai";

import Test from "../build/contracts/ERC20.json"
import { createAndFinalizeBlock, customRequest, describeWithFrontier } from "./util";
import { AbiItem } from "web3-utils";

describeWithFrontier("Frontier RPC (ERC20 Contract)", (context) => {
	const GENESIS_ACCOUNT = "0x6be02d1d3665660d22ff9624b7be0551ee1ac91b";
	const GENESIS_ACCOUNT_PRIVATE_KEY = "0x99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342";

	const TEST_CONTRACT_BYTECODE = Test.bytecode;
	const TEST_CONTRACT_ABI = Test.abi as AbiItem[];
	const FIRST_CONTRACT_ADDRESS = "0xc2bf5f29a4384b1ab0c063e1c666f02121b6084a"; // Those test are ordered. In general this should be avoided, but due to the time it takes	// to spin up a frontier node, it saves a lot of time.

	before("create the contract", async function () {
		this.timeout(15000);
		const tx = await context.web3.eth.accounts.signTransaction(
			{
				from: GENESIS_ACCOUNT,
				data: TEST_CONTRACT_BYTECODE,
				value: "0x00",
				gasPrice: "0x0001",
				gas: "0x10000000",
			},
			GENESIS_ACCOUNT_PRIVATE_KEY
		);
		await customRequest(context.web3, "eth_sendRawTransaction", [tx.rawTransaction]);
		await createAndFinalizeBlock(context.web3);
	});

	it("get transaction by hash", async () => {
		const latestBlock = await context.web3.eth.getBlock("latest");
		expect(latestBlock.transactions.length).to.equal(1);

		const tx_hash = latestBlock.transactions[0];
		const tx = await context.web3.eth.getTransaction(tx_hash);
		expect(tx.hash).to.equal(tx_hash);
	});

	it("should have 1000 balance", async function () {
		this.timeout(15000);
		const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT,
			gasPrice: "0x01",
		});

		expect(await contract.methods.balanceOf(GENESIS_ACCOUNT).call()).to.equal('1000');
	});

	it("should transfer 3 token to contract address", async function () {
		this.timeout(15000);
		const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT,
			gasPrice: "0x01",
		});
		const tx = await context.web3.eth.accounts.signTransaction(
			{
				from: GENESIS_ACCOUNT,
				to: FIRST_CONTRACT_ADDRESS,
				data: contract.methods.transfer(FIRST_CONTRACT_ADDRESS, 3).encodeABI(),
				value: "0x00",
				gasPrice: "0x01",
				gas: "0x100000",
			},
			GENESIS_ACCOUNT_PRIVATE_KEY
		);
		await customRequest(context.web3, "eth_sendRawTransaction", [tx.rawTransaction]);
		await createAndFinalizeBlock(context.web3);
		expect(await contract.methods.balanceOf(FIRST_CONTRACT_ADDRESS).call()).to.equal('3');
		expect(await contract.methods.balanceOf(GENESIS_ACCOUNT).call()).to.equal('997');
	});

});
