use crate::datatype::{
    DataKeys, EventLog, FollowCategory, FollowData, FollowError, NotificationPriority,
};
use crate::interface::AlertOperations;
use crate::NotificationPreferences;
use soroban_sdk::{Address, Env, Vec};

pub struct AlertSystem;

impl AlertOperations for AlertSystem {
    fn check_price_change(env: Env, product_id: u128, _new_price: u64) -> Result<(), FollowError> {
        // Get all users following this product
        let users = Self::get_users_following_product(&env, product_id)?;

        for user_address in users {
            let follow_key = DataKeys::FollowList(user_address.clone());
            let follows: Vec<FollowData> = env
                .storage()
                .persistent()
                .get(&follow_key)
                .unwrap_or_else(|| Vec::new(&env));

            let prefs_key = DataKeys::AlertSettings(user_address.clone());
            let user_prefs = env
                .storage()
                .persistent()
                .get::<DataKeys, NotificationPreferences>(&prefs_key);

            if user_prefs.is_none() || user_prefs.unwrap().mute_notifications {
                return Ok(());
            }

            // Check if user is following for price changes
            if let Some(_follow) = follows.iter().find(|f| {
                u128::from(f.product_id) == product_id
                    && f.categories.contains(&FollowCategory::PriceChange)
            }) {
                // Log the price change event
                let event = EventLog {
                    product_id,
                    event_type: FollowCategory::PriceChange,
                    triggered_at: env.ledger().timestamp(),
                    priority: NotificationPriority::High,
                };

                Self::log_event(&env, user_address.clone(), event)?;
            }
        }

        Ok(())
    }

    fn check_restock(env: Env, product_id: u128) -> Result<(), FollowError> {
        let users = Self::get_users_following_product(&env, product_id)?;

        for user_address in users {
            let follow_key = DataKeys::FollowList(user_address.clone());
            let follows: Vec<FollowData> = env
                .storage()
                .persistent()
                .get(&follow_key)
                .unwrap_or_else(|| Vec::new(&env));

            if let Some(_follow) = follows.iter().find(|f| {
                u128::from(f.product_id) == product_id
                    && f.categories.contains(&FollowCategory::Restock)
            }) {
                let event = EventLog {
                    product_id,
                    event_type: FollowCategory::Restock,
                    triggered_at: env.ledger().timestamp(),
                    priority: NotificationPriority::Medium,
                };

                Self::log_event(&env, user_address.clone(), event)?;
            }
        }

        Ok(())
    }

    fn check_special_offer(env: Env, product_id: u128) -> Result<(), FollowError> {
        let users = Self::get_users_following_product(&env, product_id)?;

        for user_address in users {
            let follow_key = DataKeys::FollowList(user_address.clone());
            let follows: Vec<FollowData> = env
                .storage()
                .persistent()
                .get(&follow_key)
                .unwrap_or_else(|| Vec::new(&env));

            if let Some(_follow) = follows.iter().find(|f| {
                u128::from(f.product_id) == product_id
                    && f.categories.contains(&FollowCategory::SpecialOffer)
            }) {
                let event = EventLog {
                    product_id,
                    event_type: FollowCategory::SpecialOffer,
                    triggered_at: env.ledger().timestamp(),
                    priority: NotificationPriority::Low,
                };

                Self::log_event(&env, user_address.clone(), event)?;
            }
        }

        Ok(())
    }
}

impl AlertSystem {
    // Helper function to get all users following a specific product
    fn get_users_following_product(
        env: &Env,
        product_id: u128,
    ) -> Result<Vec<Address>, FollowError> {
        let mut following_users = Vec::new(env);

        let all_users_key = DataKeys::AllUsers;
        let all_users: Vec<Address> = env
            .storage()
            .persistent()
            .get(&all_users_key)
            .unwrap_or_else(|| Vec::new(env));

        for user in all_users.iter() {
            let follow_list_key = DataKeys::FollowList(user.clone());
            if let Some(follows) = env
                .storage()
                .persistent()
                .get::<DataKeys, Vec<FollowData>>(&follow_list_key)
            {
                if follows
                    .iter()
                    .any(|f| u128::from(f.product_id) == product_id)
                {
                    following_users.push_back(user.clone());
                }
            }
        }

        Ok(following_users)
    }

    // Rate limiting
    fn check_rate_limit(env: &Env, user: &Address) -> (bool, u64) {
        let last_notification = env
            .storage()
            .persistent()
            .get::<_, u64>(&DataKeys::LastNotification(user.clone()))
            .unwrap_or(0);

        let current_time = env.ledger().timestamp();
        (current_time - last_notification > 3600, last_notification)
    }

    fn log_event(env: &Env, user: Address, event: EventLog) -> Result<(), FollowError> {
        let (boolean_value, u64_value) = Self::check_rate_limit(env, &user);
        if !boolean_value && u64_value != 0 {
            return Ok(());
        }

        // Update the last notification timestamp
        env.storage().persistent().set(
            &DataKeys::LastNotification(user.clone()),
            &env.ledger().timestamp(),
        );

        let history_key = DataKeys::NotificationHistory(user);
        let mut history: Vec<EventLog> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(env));

        history.push_back(event);

        // Limit history size (optional)
        while history.len() > 100 {
            // Keep last 100 notifications
            history.remove(0);
        }

        env.storage().persistent().set(&history_key, &history);
        Ok(())
    }
}
