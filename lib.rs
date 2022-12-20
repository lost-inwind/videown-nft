#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]
        
#[brush::contract]
pub mod videown {
    // imports from ink!
	use ink_lang::codegen::{
        EmitEvent,
        Env,
    };
	use ink_prelude::{
		string::String,
		vec,
	};
	use ink_storage::{
		traits::SpreadAllocate,
		Mapping
	};
    
    // imports from openbrush
	use brush::contracts::psp34::extensions::mintable::*;
	use brush::contracts::psp34::extensions::enumerable::*;
    
    #[ink(storage)]
    #[derive(Default, SpreadAllocate, PSP34Storage, PSP34EnumerableStorage)]
    pub struct Videown {
    	#[PSP34StorageField]
		psp34: PSP34Data,
		#[PSP34EnumerableStorageField]
		enumerable: PSP34EnumerableData,

        /// Current asks: tokenId -> (price, seller)
		asks: Mapping<Id, Balance>,
    }

	/// Event emitted when a token transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: Id,
    }

    /// Event emitted when a token approve occurs.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        id: Option<Id>,
        approved: bool,
    }

	/// Event emitted when a token trade occurs.
	#[ink(event)]
	pub struct Trade {
		#[ink(topic)]
		seller: AccountId,
		#[ink(topic)]
		buyer: AccountId,
		#[ink(topic)]
		id: Id,
		price: Balance,
	}

	impl PSP34Internal for Videown {
        fn _emit_transfer_event(&self, from: Option<AccountId>, to: Option<AccountId>, id: Id) {
            self.env().emit_event(Transfer { from, to, id });
        }

        fn _emit_approval_event(&self, from: AccountId, to: AccountId, id: Option<Id>, approved: bool) {
            self.env().emit_event(Approval { from, to, id, approved });
        }
    }

    impl PSP34Transfer for Videown {
        fn _before_token_transfer(
            &mut self,
            _from: Option<&AccountId>,
            _to: Option<&AccountId>,
            id: &Id,
        ) -> Result<(), PSP34Error> {
            if self.asks.get(&id).is_some() {
				return Err(PSP34Error::Custom(String::from("O::TokenInSale")));
			}
			Ok(())
        }
    }
    
    // Section contains default implementation without any modifications
	impl PSP34 for Videown {}
	impl PSP34Mintable for Videown {}
	impl PSP34Enumerable for Videown {}
    
    impl Videown {
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::codegen::initialize_contract(|_instance: &mut Videown|{
				_instance._mint_to(_instance.env().caller(), Id::U8(1)).expect("Can mint");
			})
        }

        #[ink(message)]
    	pub fn ask(&mut self, id: Id, price: Balance) -> Result<(), PSP34Error> {
            let owner = self._check_token_exists(&id)?;
			let caller = self.env().caller();
			if owner != caller {
                return Err(PSP34Error::Custom(String::from("O::NotTokenOwner")));
            }
			
			self.asks.insert(&id, &price);

			Ok(())
		}

		#[ink(message, payable)]
		pub fn buy(&mut self, id: Id) -> Result<(), PSP34Error> {
			let owner = self._check_token_exists(&id)?;
			let caller = self.env().caller();
			if owner == caller {
                return Err(PSP34Error::Custom(String::from("O::OwnToken")));
            }

			let transferred = self.env().transferred_value();
			let price = self.asks.get(&id).ok_or(PSP34Error::Custom(String::from("O::TokenNotInSale")))?;
			if transferred != price {
                return Err(PSP34Error::Custom(String::from("O::NotMatchPrice")));
            }
			
			// transfer native token
			if self.env().transfer(owner, price).is_err() {
				return Err(PSP34Error::Custom(String::from("O::TransferNativeTokenError")));
			}

			self.asks.remove(&id);

            // transfer nft token
            self._before_token_transfer(Some(&owner), Some(&caller), &id)?;
            self._remove_token(&owner, &id)?;
            self._do_safe_transfer_check(&owner, &owner, &caller, &id, &vec![])?;
            self._add_token(&caller, &id)?;
            self._after_token_transfer(Some(&owner), Some(&caller), &id)?;
            self._emit_transfer_event(Some(owner), Some(caller), id.clone());

			self.env().emit_event(Trade {
				seller: owner,
				buyer: caller,
				id,
				price,
			});

			Ok(())
		}

		#[ink(message)]
		pub fn cancel(&mut self, id: Id) -> Result<(), PSP34Error> {
			let owner = self._check_token_exists(&id)?;
			let caller = self.env().caller();
			if owner != caller {
                return Err(PSP34Error::Custom(String::from("O::NotTokenOwner")));
            }
			if self.asks.get(&id).is_none() {
                return Err(PSP34Error::Custom(String::from("O::NotInSale")));
            }
			
			self.asks.remove(&id);
			
			Ok(())
		}

        #[ink(message)]
    	pub fn price(&self, id: Id) -> Option<Balance> {
            self.asks.get(&id)
		}
    }
}