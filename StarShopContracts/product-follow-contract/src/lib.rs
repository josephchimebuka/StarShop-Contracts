#![no_std]
use datatype::{DataKeys, NotificationPriority};
use soroban_sdk::{
    contract, contracterror, contractimpl, symbol_short, vec, Address, BytesN, Env, Vec,
};

use crate::alerts::AlertSystem;
use crate::datatype::Error;
use crate::follow::FollowManager;
use crate::interface::{AlertOperations, NotificationOperations};
use crate::notification::NotificationSystem;

mod alerts;
mod datatype;
mod follow;
mod interface;
mod notification;

#[cfg(test)]
mod test;

pub use crate::datatype::{
    EventLog, FollowCategory, FollowData, FollowError, NotificationPreferences,
};
pub use crate::interface::FollowOperations;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ProductFollowError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    UnauthorizedAccess = 3,
}

pub trait ProductFollowTrait {
    // Core follow functionality
    fn follow_product(
        env: Env,
        user: Address,
        product_id: u32,
        categories: Vec<FollowCategory>,
    ) -> Result<(), Error>;

    fn unfollow_product(env: Env, user: Address, product_id: u32) -> Result<(), Error>;

    fn is_following(env: Env, user: Address, product_id: u32) -> bool;

    // Alert functionality
    fn notify_price_change(env: Env, product_id: u32, new_price: u64) -> Result<(), Error>;

    fn notify_restock(env: Env, product_id: u32) -> Result<(), Error>;

    fn notify_special_offer(env: Env, product_id: u32) -> Result<(), Error>;

    // Notification management
    fn set_notification_preferences(
        env: Env,
        user: Address,
        preferences: NotificationPreferences,
    ) -> Result<(), Error>;

    fn get_notification_preferences(
        env: Env,
        user: Address,
    ) -> Result<NotificationPreferences, Error>;

    fn get_notification_history(env: Env, user: Address) -> Result<Vec<EventLog>, Error>;

    // Utility functions
    fn get_followers(env: Env, product_id: u32) -> Vec<Address>;
}

#[contract]
pub struct ProductFollowContract;

#[contractimpl]
impl ProductFollowContract {
    /// Initializes the contract with an admin address
    pub fn initialize(env: Env, admin: Address) -> Result<(), ProductFollowError> {
        if env.storage().instance().has(&symbol_short!("admin")) {
            return Err(ProductFollowError::AlreadyInitialized);
        }

        admin.require_auth();
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &admin);

        env.events().publish((symbol_short!("init"),), (admin,));

        Ok(())
    }

    /// Upgrades the contract with new WASM code
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), ProductFollowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("admin"))
            .ok_or(ProductFollowError::NotInitialized)?;

        admin.require_auth();
        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        env.events()
            .publish((symbol_short!("upgrade"),), (admin, new_wasm_hash));

        Ok(())
    }

    /// Returns the current admin address
    pub fn get_admin(env: Env) -> Result<Address, ProductFollowError> {
        env.storage()
            .instance()
            .get(&symbol_short!("admin"))
            .ok_or(ProductFollowError::NotInitialized)
    }

    /// Transfers admin rights to a new address
    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), ProductFollowError> {
        let current_admin: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("admin"))
            .ok_or(ProductFollowError::NotInitialized)?;

        current_admin.require_auth();
        new_admin.require_auth();

        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);

        env.events()
            .publish((symbol_short!("adm_xfer"),), (current_admin, new_admin));

        Ok(())
    }

    pub fn register_user(env: Env, user: Address) {
        let all_users_key = DataKeys::AllUsers;
        let mut all_users: Vec<Address> = env
            .storage()
            .persistent()
            .get(&all_users_key)
            .unwrap_or_else(|| Vec::new(&env));

        if !all_users.contains(&user) {
            all_users.push_back(user.clone());

            env.storage().persistent().set(&all_users_key, &all_users)
        }

        let preferences = NotificationPreferences {
            user: user.clone(),
            categories: vec![&env, FollowCategory::PriceChange],
            mute_notifications: false,
            priority: NotificationPriority::High,
        };

        let _ = Self::set_notification_preferences(env, user, preferences);
    }

    pub fn follow_product(
        env: Env,
        user: Address,
        product_id: u32,
        categories: Vec<FollowCategory>,
    ) -> Result<(), Error> {
        let follow_manager = FollowManager::new(&env);
        follow_manager
            .add_follower(&user, product_id, &categories)
            .map_err(|e| match e {
                FollowError::AlreadyFollowing => Error::AlreadyFollowing,
                _ => Error::InvalidProduct,
            })
    }

    pub fn unfollow_product(env: Env, user: Address, product_id: u32) -> Result<(), Error> {
        let follow_manager = FollowManager::new(&env);
        follow_manager
            .remove_follower(&user, product_id)
            .map_err(|e| match e {
                FollowError::NotFollowing => Error::NotFollowing,
                _ => Error::InvalidProduct,
            })
    }

    pub fn is_following(env: Env, user: Address, product_id: u32) -> bool {
        let follow_manager = FollowManager::new(&env);
        follow_manager.is_following(&user, product_id)
    }

    pub fn notify_price_change(env: Env, product_id: u32, new_price: u64) -> Result<(), Error> {
        <AlertSystem as AlertOperations>::check_price_change(env, product_id.into(), new_price)
            .map_err(|_| Error::NotificationFailed)
    }

    pub fn notify_restock(env: Env, product_id: u32) -> Result<(), Error> {
        <AlertSystem as AlertOperations>::check_restock(env, product_id.into())
            .map_err(|_| Error::NotificationFailed)
    }

    pub fn notify_special_offer(env: Env, product_id: u32) -> Result<(), Error> {
        <AlertSystem as AlertOperations>::check_special_offer(env, product_id.into())
            .map_err(|_| Error::NotificationFailed)
    }

    pub fn set_notification_preferences(
        env: Env,
        user: Address,
        preferences: NotificationPreferences,
    ) -> Result<(), Error> {
        <NotificationSystem as NotificationOperations>::set_notification_preferences(
            env,
            user,
            preferences,
        )
        .map_err(|_| Error::NotificationFailed)
    }

    pub fn get_notification_preferences(
        env: Env,
        user: Address,
    ) -> Result<NotificationPreferences, Error> {
        <NotificationSystem as NotificationOperations>::get_notification_preferences(env, user)
            .map_err(|_| Error::NotificationFailed)
    }

    pub fn get_notification_history(env: Env, user: Address) -> Result<Vec<EventLog>, Error> {
        <NotificationSystem as NotificationOperations>::get_notification_history(env, user)
            .map_err(|_| Error::NotificationFailed)
    }

    pub fn get_followers(env: Env, product_id: u32) -> Vec<Address> {
        let follow_manager = FollowManager::new(&env);
        let follows = follow_manager.get_followers(product_id);
        let mut addresses = Vec::new(&env);
        for follow in follows.iter() {
            addresses.push_back(follow.user.clone());
        }
        addresses
    }
}
