use crate::datatype::{DataKeys, FollowCategory, FollowData, FollowError};
use crate::interface::FollowOperations;
use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

// This is a struct that implements the FollowOperations trait
#[allow(dead_code)]
pub struct FollowSystem;

#[allow(dead_code)]
pub const DEFAULT_FOLLOW_LIMIT: u32 = 100;

pub struct FollowManager<'a> {
    env: &'a Env,
}

impl<'a> FollowManager<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self { env }
    }

    pub fn add_follower(
        &self,
        user: &Address,
        product_id: u32,
        categories: &Vec<FollowCategory>,
    ) -> Result<(), FollowError> {
        user.require_auth();

        // Validate follow limit
        let follows = self.get_followers(product_id);
        if (follows.len() as u32) >= DEFAULT_FOLLOW_LIMIT {
            return Err(FollowError::FollowLimitExceeded);
        }

        // Validate categories
        for category in categories.iter() {
            match category {
                FollowCategory::PriceChange
                | FollowCategory::Restock
                | FollowCategory::SpecialOffer => continue,
            }
        }

        let key = symbol_short!("followers");
        let mut followers = self.get_followers(product_id);

        // Check if user is already following
        if followers.iter().any(|f| &f.user == user) {
            return Err(FollowError::AlreadyFollowing);
        }

        let follow_data = FollowData {
            user: user.clone(),
            product_id,
            categories: categories.clone(),
            timestamp: self.env.ledger().timestamp(),
            expires_at: None,
        };

        // Add new follower
        followers.push_back(follow_data.clone());

        // Update the follow list of a single user
        let list_of_single_user_key = DataKeys::FollowList(user.clone());
        let mut all_follow_list_of_single_user = self
            .env
            .storage()
            .persistent()
            .get::<DataKeys, Vec<FollowData>>(&list_of_single_user_key)
            .unwrap_or(Vec::new(self.env));

        all_follow_list_of_single_user.push_back(follow_data);

        self.env
            .storage()
            .persistent()
            .set(&list_of_single_user_key, &all_follow_list_of_single_user);

        // Store updated followers
        self.env.storage().persistent().set(&key, &followers);
        Ok(())
    }

    pub fn get_followers(&self, _product_id: u32) -> Vec<FollowData> {
        let key = symbol_short!("followers");
        self.env
            .storage()
            .persistent()
            .get::<_, Vec<FollowData>>(&key)
            .unwrap_or_else(|| Vec::new(self.env))
    }

    pub fn remove_follower(&self, user: &Address, product_id: u32) -> Result<(), FollowError> {
        user.require_auth();

        let key = symbol_short!("followers");
        let follows: Vec<FollowData> = self
            .env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(self.env));

        let mut updated_follows = Vec::new(self.env);
        let mut found = false;

        for follow in follows.iter() {
            if &follow.user != user || follow.product_id != product_id {
                updated_follows.push_back(follow.clone());
            } else {
                found = true;
            }
        }

        if !found {
            return Err(FollowError::NotFollowing);
        }

        self.env.storage().persistent().set(&key, &updated_follows);
        Ok(())
    }

    pub fn is_following(&self, user: &Address, product_id: u32) -> bool {
        let follows = self.get_followers(product_id);
        follows.iter().any(|f| &f.user == user)
    }

    #[allow(dead_code)]
    pub fn get_follow_categories(&self, user: &Address, product_id: u32) -> Vec<FollowCategory> {
        let follows = self.get_followers(product_id);

        if let Some(follow_data) = follows
            .iter()
            .find(|f| &f.user == user && f.product_id == product_id)
        {
            follow_data.categories.clone()
        } else {
            Vec::new(self.env)
        }
    }

    #[allow(dead_code)]
    fn get_storage_key(&self, _product_id: u32) -> Symbol {
        symbol_short!("followers")
    }
}

// Implementation for FollowOperations
impl FollowOperations for FollowSystem {
    fn follow_product(
        env: Env,
        user: Address,
        product_id: u128,
        categories: Vec<FollowCategory>,
    ) -> Result<(), FollowError> {
        let product_id_u32: u32 = product_id
            .try_into()
            .map_err(|_| FollowError::InvalidProductId)?;
        let manager = FollowManager::new(&env);
        manager.add_follower(&user, product_id_u32, &categories)
    }

    fn unfollow_product(env: Env, user: Address, product_id: u128) -> Result<(), FollowError> {
        let product_id_u32: u32 = product_id
            .try_into()
            .map_err(|_| FollowError::InvalidProductId)?;
        let manager = FollowManager::new(&env);
        manager.remove_follower(&user, product_id_u32)
    }

    fn get_followed_products(env: Env, _user: Address) -> Result<Vec<FollowData>, FollowError> {
        let manager = FollowManager::new(&env);
        Ok(manager.get_followers(0))
    }
}
