use scrypto::prelude::*;

#[derive(ScryptoSbor)]
pub struct UsdToken {
    pub vault: FungibleVault,
    pub accepted: bool,
    pub usd_per_fusd: Decimal,
    pub fusd_minted: Decimal,
}

/// Status of a CDP
#[derive(ScryptoSbor, Clone, Debug, PartialEq)]
pub enum CdpStatus {
    Active,
    Closed,
    Liquidated,
}

/// Data struct of a loan receipt / CDP receipt, gained when opening a CDP / loan
#[derive(ScryptoSbor, NonFungibleData, Clone, Debug)]
pub struct Cdp {
    /// Image of the NFT
    #[mutable]
    pub key_image_url: Url,
    /// The resource address of the collateral used for this loan / CDP.
    pub collateral_address: ResourceAddress,
    /// The current amount of collateral deposited in this CDP.
    #[mutable]
    pub collateral_amount: Decimal,
    /// The amount of debt denominated in the pool's internal unit (before applying the debt multiplier).
    #[mutable]
    pub pool_debt: Decimal,
    /// The ratio of collateral amount to pool debt (collateral_amount / pool_debt). Used for sorting CDPs.
    #[mutable]
    pub collateral_fusd_ratio: Decimal,
    /// The selected annual interest rate for this CDP. A rate of -420 indicates a privileged, irredeemable loan.
    #[mutable]
    pub interest: Decimal,
    /// Timestamp of the last time the interest rate for this CDP was changed.
    #[mutable]
    pub last_interest_change: Instant,
    /// The current status of the CDP / loan.
    #[mutable]
    pub status: CdpStatus,
    /// Optional local ID of the privileged borrower NFT linked to this CDP.
    #[mutable]
    pub privileged_borrower: Option<NonFungibleLocalId>,
}

/// A structure for returning stability pool information, including current asset amounts.
#[derive(ScryptoSbor, Clone)]
pub struct StabilityPoolInfoReturn {
    /// The `ResourceAddress` of the collateral associated with this pool.
    pub collateral: ResourceAddress,
    /// The configured payout split (optional override).
    pub payout_split: Option<Decimal>,
    /// The configured liquidity rewards split (optional override).
    pub liquidity_rewards_split: Option<Decimal>,
    /// The configured stability pool split (optional override).
    pub stability_pool_split: Option<Decimal>,
    /// Flag indicating if direct collateral buys are allowed.
    pub allow_pool_buys: bool,
    /// The configured buy price modifier (optional override).
    pub pool_buy_price_modifier: Option<Decimal>,
    /// The current amount of accumulated fUSD liquidity rewards.
    pub liquidity_rewards: Decimal,
    /// Global reference to the underlying `TwoResourcePool` component.
    pub pool: Global<TwoResourcePool>,
    /// The current amount of collateral held within the pool.
    pub collateral_amount: Decimal,
    /// The current amount of fUSD held within the pool.
    pub fusd_amount: Decimal,
    /// The recent history of lowest active interest rates.
    pub latest_lowest_interests: Vec<Decimal>,
    /// Timestamp of the last update to the interest history.
    pub last_lowest_interests_update: Instant,
}

#[blueprint]
#[types(UsdToken, FungibleVault, ResourceAddress, NonFungibleLocalId)]
mod flux_addition {
    enable_method_auth! {
        methods {
            mint_with_usd => PUBLIC;
            redeem_with_fusd => PUBLIC;
            close_loan => PUBLIC;
            receive_badges => PUBLIC;
            partial_liquidate_cdp => PUBLIC;
            put_usd_in_vault => PUBLIC;
            retrieve_collateral => PUBLIC;
            get_usd_amount_in_vault => PUBLIC;
            get_collateral_price => PUBLIC;
            set_usd_per_fusd => restrict_to: [OWNER];
            add_usd_token => restrict_to: [OWNER];
            toggle_usd_token_accepted => restrict_to: [OWNER];
            set_max_debt_before_close => restrict_to: [OWNER];
            set_fine => restrict_to: [OWNER];
            send_badges => restrict_to: [OWNER];
            set_oracle => restrict_to: [OWNER];
            take_usd_from_vault => restrict_to: [OWNER];
            retrieve_collateral_admin => restrict_to: [OWNER];
        }
    }

    extern_blueprint! {
        "package_rdx1p55x9av2cu7re0f2l044ednmznhj4rnm8y9cnpzstx4pemdglxu696", //mainnet
        //"package_tdx_2_1phluu3kccm6h30qj83z7tjxgj5yk0ppmx8w7tma4cmfhgv9upnv2wj", //stokenet
        Flux {
            fn free_fusd(&self, amount: Decimal) -> Bucket;
            fn close_cdp(&self, cdp_id: NonFungibleLocalId, fusd_payment: Bucket) -> (Bucket, Bucket);
            fn partial_close_cdp(&self, cdp_id: NonFungibleLocalId, repayment: Bucket) -> (Option<Bucket>, Option<Bucket>);
            fn remove_collateral(&self, cdp_id: NonFungibleLocalId, amount: Decimal, with_price: Option<Decimal>) -> Bucket;
            fn check_liquidate_cdp(&self, cdp_id: NonFungibleLocalId, with_price: Option<Decimal>) -> (bool, Decimal, ResourceAddress);
        }
    }

    extern_blueprint! {
        "package_rdx1p55x9av2cu7re0f2l044ednmznhj4rnm8y9cnpzstx4pemdglxu696", //mainnet
        //"package_tdx_2_1phluu3kccm6h30qj83z7tjxgj5yk0ppmx8w7tma4cmfhgv9upnv2wj", //stokenet
        StabilityPools {
            fn get_stability_pool_infos(&self, resource_addresses: Option<Vec<ResourceAddress>>) -> Vec<StabilityPoolInfoReturn>;
        }
    }

    const FLUX: Global<Flux> = global_component!(
        Flux,
        "component_rdx1czgv2hx5lq4v5tjm32u69s5dw8ja0d4qeau2y5vktvaxlrmsfdy08u" //mainnet
                                                                               //"component_tdx_2_1cra7c4mxf5e7tzhu6au8jax8sx4klt5kkexghwr789nyp735ktycfl" //stokenet
    );

    const STABILITY_POOLS: Global<StabilityPools> = global_component!(
        StabilityPools,
        "component_rdx1cpkye6pp2643ghalcppdxks6kymyu5gla87gf7sk34k0vg7xu57jaj" //mainnet
                                                                               //"component_tdx_2_1cr56m97gp0jv3wvnxlaxmnuvuvdq9vlrc2stzk0xfa5cmytcr83gfm" //stokenet
    );

    struct FluxAddition {
        usd_tokens: KeyValueStore<ResourceAddress, UsdToken>,
        retrievable_collateral: KeyValueStore<NonFungibleLocalId, FungibleVault>,
        cdp_address: ResourceAddress,
        fine: Decimal,
        max_debt_before_close: Decimal,
        badge_vault: FungibleVault,
        oracle: Global<AnyComponent>,
        oracle_method_name: String,
        fusd_address: ResourceAddress,
    }

    impl FluxAddition {
        pub fn instantiate(
            cdp_address: ResourceAddress,
            fusd_address: ResourceAddress,
            badge_address: ResourceAddress,
            controller_address: ResourceAddress,
            oracle_address: ComponentAddress,
            oracle_method_name: String,
            initial_usd_token: ResourceAddress,
            initial_usd_per_fusd: Decimal,
        ) -> Global<FluxAddition> {
            let usd_tokens: KeyValueStore<ResourceAddress, UsdToken> =
                KeyValueStore::new_with_registered_type();

            let usd_token = UsdToken {
                vault: FungibleVault::new(initial_usd_token),
                accepted: true,
                usd_per_fusd: initial_usd_per_fusd,
                fusd_minted: Decimal::ZERO,
            };

            usd_tokens.insert(initial_usd_token, usd_token);

            Self {
                usd_tokens,
                retrievable_collateral: KeyValueStore::new_with_registered_type(),
                cdp_address,
                fine: dec!("1.1"),
                max_debt_before_close: dec!("0.1"),
                fusd_address,
                badge_vault: FungibleVault::new(badge_address),
                oracle: Global::from(oracle_address),
                oracle_method_name,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(controller_address))))
            .globalize()
        }

        pub fn add_usd_token(&mut self, token: ResourceAddress, usd_per_fusd: Decimal) {
            assert!(
                usd_per_fusd > Decimal::ZERO,
                "usd_per_fusd must be positive"
            );
            assert!(
                self.usd_tokens.get(&token).is_none(),
                "Vault Already Exists"
            );

            let usd_token = UsdToken {
                vault: FungibleVault::new(token),
                accepted: true,
                usd_per_fusd,
                fusd_minted: Decimal::ZERO,
            };

            self.usd_tokens.insert(token, usd_token);
        }

        pub fn toggle_usd_token_accepted(&mut self, token: ResourceAddress) {
            let mut usd_token = self.usd_tokens.get_mut(&token).unwrap();
            usd_token.accepted = !usd_token.accepted;
        }

        pub fn mint_with_usd(&mut self, usd: FungibleBucket) -> Bucket {
            let token = usd.resource_address();
            let mut usd_token = self.usd_tokens.get_mut(&token).unwrap();

            assert!(usd_token.accepted, "Token not accepted");

            let fusd_amount = usd.amount() / usd_token.usd_per_fusd;
            let badge_amount = self.badge_vault.amount();
            let fusd = self
                .badge_vault
                .authorize_with_amount(badge_amount, || FLUX.free_fusd(fusd_amount));

            usd_token.vault.put(usd);
            usd_token.fusd_minted += fusd.amount();

            fusd
        }

        pub fn redeem_with_fusd(
            &mut self,
            mut fusd: FungibleBucket,
            against_token: ResourceAddress,
        ) -> (FungibleBucket, FungibleBucket) {
            let mut usd_token = self.usd_tokens.get_mut(&against_token).unwrap();
            assert!(
                usd_token.fusd_minted > Decimal::ZERO,
                "No fUSD minted against this token"
            );
            let max_usd_to_receive: Decimal = usd_token.vault.amount();
            let usd_per_fusd: Decimal = usd_token.vault.amount() / usd_token.fusd_minted;
            let usd_to_receive = usd_per_fusd * fusd.amount();

            let badge_amount = self.badge_vault.amount();
            if usd_to_receive > max_usd_to_receive {
                let fusd_to_take = max_usd_to_receive / usd_per_fusd;

                self.badge_vault.authorize_with_amount(badge_amount, || {
                    fusd.take(fusd_to_take).burn();
                });

                usd_token.fusd_minted -= fusd_to_take;

                (usd_token.vault.take(max_usd_to_receive), fusd)
            } else {
                let fusd_amount = fusd.amount();
                self.badge_vault.authorize_with_amount(badge_amount, || {
                    fusd.take(fusd_amount).burn();
                });

                usd_token.fusd_minted -= fusd_amount;

                (usd_token.vault.take(usd_to_receive), fusd)
            }
        }

        pub fn set_usd_per_fusd(&mut self, token: ResourceAddress, usd_per_fusd: Decimal) {
            assert!(
                usd_per_fusd > Decimal::ZERO,
                "usd_per_fusd must be positive"
            );
            self.usd_tokens.get_mut(&token).unwrap().usd_per_fusd = usd_per_fusd;
        }

        pub fn set_fine(&mut self, fine: Decimal) {
            assert!(fine >= Decimal::ONE, "Fine cannot be below 1");
            self.fine = fine;
        }

        pub fn set_max_debt_before_close(&mut self, max_debt_before_close: Decimal) {
            self.max_debt_before_close = max_debt_before_close;
        }

        pub fn close_loan(
            &mut self,
            cdp_id: NonFungibleLocalId,
            fusd: Bucket,
            message: String,
            signature: String,
        ) -> (Bucket, Option<Bucket>) {
            let cdp_manager: NonFungibleResourceManager =
                NonFungibleResourceManager::from(self.cdp_address);
            let receipt_data: Cdp = cdp_manager.get_non_fungible_data(&cdp_id);

            let collateral_price: Decimal = self.oracle.call_raw(
                &self.oracle_method_name,
                scrypto_args!(receipt_data.collateral_address, message, signature),
            );

            let fusd_input_amount: Decimal = fusd.amount();
            let badge_amount = self.badge_vault.amount();

            let (mut collateral, leftover_fusd): (Bucket, Bucket) = self
                .badge_vault
                .authorize_with_amount(badge_amount, || FLUX.close_cdp(cdp_id.clone(), fusd));
            let fusd_spent: Decimal = fusd_input_amount - leftover_fusd.amount();

            assert!(
                fusd_spent < self.max_debt_before_close,
                "Too much debt to forcibly close."
            );

            let collateral_reward_max = (fusd_spent / collateral_price) * self.fine;
            let collateral_surplus = collateral.amount() - collateral_reward_max;

            if collateral_surplus > Decimal::ZERO {
                let surplus_bucket = collateral.take(collateral_surplus);
                self.put_retrievable_collateral(cdp_id, surplus_bucket.as_fungible());
            }

            if leftover_fusd.amount() > Decimal::ZERO {
                (collateral, Some(leftover_fusd))
            } else {
                leftover_fusd.drop_empty();
                (collateral, None)
            }
        }

        pub fn partial_liquidate_cdp(
            &mut self,
            cdp_id: NonFungibleLocalId,
            fusd: Bucket,
            message: String,
            signature: String,
            price_multiplier_for_removal: Decimal,
        ) -> (Bucket, Bucket) {
            let cdp_manager: NonFungibleResourceManager =
                NonFungibleResourceManager::from(self.cdp_address);
            let receipt_data: Cdp = cdp_manager.get_non_fungible_data(&cdp_id);

            let stability_pool_infos = STABILITY_POOLS
                .get_stability_pool_infos(Some(vec![receipt_data.collateral_address]));
            let fusd_in_stability_pool: Decimal = stability_pool_infos
                .iter()
                .find(|info| info.collateral == receipt_data.collateral_address)
                .map(|info| info.fusd_amount)
                .unwrap_or(Decimal::ZERO);

            let collateral_price: Decimal = self.oracle.call_raw(
                &self.oracle_method_name,
                scrypto_args!(receipt_data.collateral_address, message, signature),
            );

            let (liquidatable, _real_debt, _collateral_address) =
                FLUX.check_liquidate_cdp(cdp_id.clone(), Some(collateral_price));
            assert!(liquidatable, "This CDP cannot be liquidated");

            let fusd_input_amount: Decimal = fusd.amount();
            let badge_amount = self.badge_vault.amount();

            let (collateral, leftover_fusd): (Option<Bucket>, Option<Bucket>) =
                self.badge_vault.authorize_with_amount(badge_amount, || {
                    FLUX.partial_close_cdp(cdp_id.clone(), fusd)
                });

            let leftover_fusd_bucket = match leftover_fusd {
                Some(leftover_fusd) => leftover_fusd,
                None => Bucket::new(self.fusd_address),
            };
            let leftover_fusd_amount: Decimal = leftover_fusd_bucket.amount();

            let fusd_spent = fusd_input_amount - leftover_fusd_amount;
            assert!(
                fusd_spent > fusd_in_stability_pool,
                "Enough fUSD in stability pool to liquidate"
            );

            let mut collateral_bucket = match collateral {
                Some(collateral) => collateral,
                None => Bucket::new(receipt_data.collateral_address),
            };
            let collateral_amount: Decimal = collateral_bucket.amount();

            let max_collateral_to_take = (fusd_spent / collateral_price) * self.fine;
            let collateral_shortage = max_collateral_to_take - collateral_amount;
            let receipt_data_after_close: Cdp = cdp_manager.get_non_fungible_data(&cdp_id);
            let collateral_to_take_out_to_reach_min =
                receipt_data_after_close.collateral_amount - (Decimal::ONE / collateral_price); //collateral available - min collateral in cdp (we want at least 1 dollar in there)

            let extra_collateral: Bucket = if collateral_shortage > Decimal::ZERO {
                let amount_to_remove =
                    collateral_shortage.min(collateral_to_take_out_to_reach_min.max(Decimal::ZERO));

                let high_price = collateral_price * price_multiplier_for_removal;
                if amount_to_remove > Decimal::ZERO {
                    self.badge_vault.authorize_with_amount(badge_amount, || {
                        FLUX.remove_collateral(cdp_id.clone(), amount_to_remove, Some(high_price))
                    })
                } else {
                    Bucket::new(receipt_data.collateral_address)
                }
            } else if collateral_shortage < Decimal::ZERO {
                let collateral_surplus = collateral_shortage.checked_abs().unwrap();
                self.put_retrievable_collateral(
                    cdp_id,
                    collateral_bucket.take(collateral_surplus).as_fungible(),
                );
                Bucket::new(receipt_data.collateral_address)
            } else {
                Bucket::new(receipt_data.collateral_address)
            };

            collateral_bucket.put(extra_collateral);

            (collateral_bucket, leftover_fusd_bucket)
        }

        pub fn receive_badges(&mut self, badge_bucket: Bucket) {
            self.badge_vault.put(badge_bucket.as_fungible());
        }

        pub fn set_oracle(&mut self, oracle_address: ComponentAddress, single_method_name: String) {
            self.oracle = Global::from(oracle_address);
            self.oracle_method_name = single_method_name;
        }

        pub fn send_badges(&mut self, amount: Decimal, receiver_address: ComponentAddress) {
            let receiver: Global<AnyComponent> = Global::from(receiver_address);
            let badge_bucket: Bucket = self.badge_vault.take(amount).into();
            receiver.call_raw("receive_badges", scrypto_args!(badge_bucket))
        }

        pub fn take_usd_from_vault(
            &mut self,
            usd_token: ResourceAddress,
            amount: Decimal,
        ) -> FungibleBucket {
            self.usd_tokens
                .get_mut(&usd_token)
                .unwrap()
                .vault
                .take(amount)
        }

        // this is public, if you want to put money in a vault you cannot take out, feel free to
        pub fn put_usd_in_vault(&mut self, usd_token: ResourceAddress, usd_bucket: FungibleBucket) {
            self.usd_tokens
                .get_mut(&usd_token)
                .unwrap()
                .vault
                .put(usd_bucket);
        }

        pub fn retrieve_collateral(&mut self, cdp_proof: NonFungibleProof) -> FungibleBucket {
            let cdp_proof = cdp_proof.check_with_message(
                self.cdp_address,
                "Incorrect proof! Are you sure this loan is yours?",
            );

            let cdp = cdp_proof.non_fungible::<Cdp>();
            let cdp_id: NonFungibleLocalId = cdp.local_id().clone();

            self.retrievable_collateral
                .get_mut(&cdp_id)
                .unwrap()
                .take_all()
        }

        // admin should be able to recover collateral, for if the user burns their receipt before redeeming here, accidentally
        // this is not that much of an issue, since the admin will be the DAO. the DAO already has access to all funds and is obviously decentralized.
        pub fn retrieve_collateral_admin(&mut self, cdp_id: NonFungibleLocalId) -> FungibleBucket {
            self.retrievable_collateral
                .get_mut(&cdp_id)
                .unwrap()
                .take_all()
        }

        pub fn get_usd_amount_in_vault(&mut self, usd_token: ResourceAddress) -> Decimal {
            self.usd_tokens.get_mut(&usd_token).unwrap().vault.amount()
        }

        pub fn get_collateral_price(
            &mut self,
            collateral: ResourceAddress,
            message: String,
            signature: String,
        ) -> Decimal {
            self.oracle.call_raw(
                &self.oracle_method_name,
                scrypto_args!(collateral, message, signature),
            )
        }

        fn put_retrievable_collateral(
            &mut self,
            cdp_id: NonFungibleLocalId,
            bucket: FungibleBucket,
        ) {
            if self.retrievable_collateral.get(&cdp_id).is_some() {
                self.retrievable_collateral
                    .get_mut(&cdp_id)
                    .unwrap()
                    .put(bucket);
            } else {
                self.retrievable_collateral
                    .insert(cdp_id, FungibleVault::with_bucket(bucket));
            }
        }
    }
}
