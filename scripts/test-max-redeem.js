const { Contract, Address, SorobanRpc } = require('stellar-sdk');

const VAULT_ADDRESS = 'CDGSSWMTWASCHMRPPWZ2YQJOSARYJRIFJDNNA2COB35BXACESZE7RXHQ';
const USER_ADDRESS = 'GAPLXXBHMVBKUNOOOMR2OFHGSWNN5MYFJRMZOECOT6IKY5RV3UGWFVG7';
const RPC_URL = 'https://rpc.lightsail.network/';

async function testMaxRedeem() {
    const server = new SorobanRpc.Server(RPC_URL);
    const contract = new Contract(VAULT_ADDRESS);

    console.log('Testing max_redeem and preview functions...\n');

    try {
        // Get account for building tx
        const sourceAccount = await server.getAccount(USER_ADDRESS);

        // Test max_redeem
        const maxRedeemTx = new SorobanRpc.TransactionBuilder(sourceAccount, {
            fee: '100',
            networkPassphrase: 'Public Global Stellar Network ; September 2015'
        })
            .addOperation(contract.call('max_redeem', Address.fromString(USER_ADDRESS).toScVal()))
            .setTimeout(30)
            .build();

        const maxRedeemResult = await server.simulateTransaction(maxRedeemTx);
        console.log('max_redeem result:', maxRedeemResult);

        if (maxRedeemResult.result?.retval) {
            const maxRedeem = BigInt(maxRedeemResult.result.retval._value?._value || 0);
            console.log('Max redeemable shares:', maxRedeem.toString(), '=', Number(maxRedeem) / 10_000_000);
        }

        // Test preview_redeem with full shares
        const shares = 20000300000n;
        const previewTx = new SorobanRpc.TransactionBuilder(sourceAccount, {
            fee: '100',
            networkPassphrase: 'Public Global Stellar Network ; September 2015'
        })
            .addOperation(contract.call('preview_redeem', Address.fromString(USER_ADDRESS).toScVal()))
            .setTimeout(30)
            .build();

        console.log('\nTrying to call preview_redeem...');

    } catch (error) {
        console.error('Error:', error);
    }
}

testMaxRedeem();
