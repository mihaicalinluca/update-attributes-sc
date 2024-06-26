#![allow(non_snake_case)]

mod proxy;

use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::sdk;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

const GATEWAY: &str = sdk::blockchain::TESTNET_GATEWAY;
const STATE_FILE: &str = "state.toml";

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let mut interact = ContractInteract::new().await;
    match cmd.as_str() {
        "deploy" => interact.deploy().await,
        "issue" => interact.issue().await,
        "create" => interact.create().await,
        "update" => interact.update().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    contract_address: Option<Bech32Address>,
}

impl State {
    // Deserializes state from file
    pub fn load_state() -> Self {
        if Path::new(STATE_FILE).exists() {
            let mut file = std::fs::File::open(STATE_FILE).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            toml::from_str(&content).unwrap()
        } else {
            Self::default()
        }
    }

    /// Sets the contract address
    pub fn set_address(&mut self, address: Bech32Address) {
        self.contract_address = Some(address);
    }

    /// Returns the contract address
    pub fn current_address(&self) -> &Bech32Address {
        self.contract_address
            .as_ref()
            .expect("no known contract, deploy first")
    }
}

impl Drop for State {
    // Serializes state to file
    fn drop(&mut self) {
        let mut file = std::fs::File::create(STATE_FILE).unwrap();
        file.write_all(toml::to_string(self).unwrap().as_bytes())
            .unwrap();
    }
}

struct ContractInteract {
    interactor: Interactor,
    wallet_address: Address,
    second_user: Address,
    contract_code: BytesValue,
    state: State,
}

impl ContractInteract {
    async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let wallet_address = interactor.register_wallet(test_wallets::alice());
        let second_user = interactor.register_wallet(
            sdk::wallet::Wallet::from_pem_file_contents("-----BEGIN PRIVATE KEY for erd1fs0p347knaqdl8xgy0ya9ygpuegddatl0g45sekwwgzndw8za7pqskjf64-----
OTliYzE2YjQ2ZDM0YmY1NjI4MTI1NTJkYWQ5OGRkZDdiY2I4YjlkZDFlODAxOTU5
OWUzYWQzMGRlNmUyMzA2MjRjMWUxOGQ3ZDY5ZjQwZGY5Y2M4MjNjOWQyOTEwMWU2
NTBkNmY1N2Y3YTJiNDg2NmNlNzIwNTM2YjhlMmVmODI=
-----END PRIVATE KEY for erd1fs0p347knaqdl8xgy0ya9ygpuegddatl0g45sekwwgzndw8za7pqskjf64-----".to_string()).unwrap(),
        );

        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/update-attributes.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            wallet_address,
            second_user,
            contract_code,
            state: State::load_state(),
        }
    }

    async fn deploy(&mut self) {
        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(200_000_000u64)
            .typed(proxy::UpdateAttributesProxy)
            .init()
            .code(&self.contract_code)
            .code_metadata(CodeMetadata::PAYABLE)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state.set_address(Bech32Address::from_bech32_string(
            new_address_bech32.clone(),
        ));

        println!("new address: {new_address_bech32}");
    }

    async fn issue(&mut self) {
        let egld_amount = BigUint::<StaticApi>::from(50000000000000000u64);
        let token_name = ManagedBuffer::new_from_bytes(&b"Whatever"[..]);
        let token_ticker = ManagedBuffer::new_from_bytes(&b"TESTNFT"[..]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(200_000_000u64)
            .typed(proxy::UpdateAttributesProxy)
            .issue(token_name, token_ticker)
            .egld(egld_amount)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn create(&mut self) {
        let send_to = Bech32Address::from_bech32_string(
            "erd1fs0p347knaqdl8xgy0ya9ygpuegddatl0g45sekwwgzndw8za7pqskjf64".to_string(),
        );
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(200_000_000u64)
            .typed(proxy::UpdateAttributesProxy)
            .create(send_to)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn update(&mut self) {
        let new_attributes = ManagedBuffer::new_from_bytes(&b"NEWATTRIBUTES"[..]);
        let token_identifier = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::UpdateAttributesProxy)
            .nft_token_id()
            .returns(ReturnsResult)
            .prepare_async()
            .run()
            .await;

        let response = self
            .interactor
            .tx()
            .from(&self.second_user)
            .to(self.state.current_address())
            .gas(200_000_000u64)
            .typed(proxy::UpdateAttributesProxy)
            .update(new_attributes)
            .single_esdt(&token_identifier, 1u64, &BigUint::from(1u64))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
}
