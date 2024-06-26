#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;

/// An empty contract. To be used as a template when starting a new contract from scratch.
#[multiversx_sc::contract]
pub trait UpdateAttributes {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("EGLD")]
    #[endpoint]
    fn issue(&self, token_name: ManagedBuffer, token_ticker: ManagedBuffer) {
        require!(self.nft_token_id().is_empty(), "Token already issued");

        let payment_amount = self.call_value().egld_value().clone_value();
        self.send()
            .esdt_system_sc_proxy()
            .issue_and_set_all_roles(
                payment_amount,
                token_name,
                token_ticker,
                EsdtTokenType::NonFungible,
                0,
            )
            .with_callback(self.callbacks().issue_callback())
            .async_call_and_exit()
    }

    #[endpoint]
    fn create(&self, to: ManagedAddress) {
        let nonce = self.send().esdt_nft_create(
            &self.nft_token_id().get(),
            &BigUint::from(1u8),
            &ManagedBuffer::new(),
            &BigUint::from(0u8),
            &ManagedBuffer::new(),
            &ManagedBuffer::from(b"common"),
            &ManagedVec::new(),
        );

        self.tx()
            .to(to)
            .single_esdt(&self.nft_token_id().get(), nonce, &BigUint::from(1u64))
            .transfer_execute();
    }

    #[payable("*")]
    #[endpoint]
    fn update(&self, new_attributes: ManagedBuffer) {
        let token = self.call_value().single_esdt();

        self.tx()
            .to(ToSelf)
            .typed(system_proxy::UserBuiltinProxy)
            .nft_update_attributes(&token.token_identifier, token.token_nonce, &new_attributes)
            .sync_call();

        self.tx()
            .to(ToCaller)
            .single_esdt(&token.token_identifier, token.token_nonce, &BigUint::from(1u64))
            .transfer();
    }

    #[endpoint]
    fn send_nft(&self, to: ManagedAddress, nonce: u64) {
        self.tx()
            .to(to)
            .single_esdt(&self.nft_token_id().get(), nonce, &BigUint::from(1u64))
            .transfer();
    }

    #[callback]
    fn issue_callback(
        &self,
        #[call_result] result: ManagedAsyncCallResult<EgldOrEsdtTokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.nft_token_id().set(&token_id.unwrap_esdt());
            }
            ManagedAsyncCallResult::Err(_) => {
                let returned = self.call_value().egld_or_single_esdt();
                if returned.token_identifier.is_egld() && returned.amount > 0 {
                    self.tx().to(ToCaller).egld(returned.amount).transfer();
                }
            }
        }
    }

    #[view]
    #[storage_mapper("nftTokenId")]
    fn nft_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
