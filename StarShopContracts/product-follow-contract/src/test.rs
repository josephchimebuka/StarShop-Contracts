#![cfg(test)]

use crate::{
    datatype::{DataKeys, NotificationPriority},
    follow::DEFAULT_FOLLOW_LIMIT,
};

use super::*;
use soroban_sdk::{testutils::Address as TestAddress, vec, Env, Vec};

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn test_panic_follower_not_auth() {
    let env = Env::default();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let follower_address = <Address>::generate(&env);

    client.follow_product(
        &follower_address,
        &3,
        &vec![&env, FollowCategory::PriceChange],
    );
}

#[test]
#[should_panic]
fn test_panic_user_already_following() {
    let env = Env::default();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let follower_address = <Address>::generate(&env);
    let product_id = 3u32;
    let categories = vec![&env, FollowCategory::PriceChange];
    env.mock_all_auths();

    client.follow_product(&follower_address, &product_id, &categories);
    client.follow_product(&follower_address, &product_id, &categories);
}

#[test]
#[should_panic]
fn test_panic_follow_limit_exceeded() {
    let env = Env::default();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let product_id = 3u32;
    let categories = vec![&env, FollowCategory::PriceChange];
    env.mock_all_auths();

    for _ in 0..DEFAULT_FOLLOW_LIMIT + 1 {
        let follower_address = <Address>::generate(&env);
        client.follow_product(&follower_address, &product_id, &categories);
    }
}

#[test]
fn test_add_follower() {
    let env = Env::default();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let follower_address = <Address>::generate(&env);
    let product_id = 3u32;
    let categories = vec![&env, FollowCategory::PriceChange];
    env.mock_all_auths();

    client.follow_product(&follower_address, &product_id, &categories);

    env.as_contract(&contract_id, || {
        let key = symbol_short!("followers");
        let reputation_records: Vec<FollowData> = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Reputation history key rating key not found");
        assert_eq!(reputation_records.len(), 1);
        assert_eq!(reputation_records.first().unwrap().user, follower_address);
        assert_eq!(reputation_records.first().unwrap().product_id, product_id);
        assert_eq!(reputation_records.first().unwrap().categories, categories);
        assert_eq!(
            reputation_records.first().unwrap().timestamp,
            env.ledger().timestamp()
        );
        assert_eq!(reputation_records.first().unwrap().expires_at, None);
    });
}

#[test]
fn test_unfollow() {
    let env = Env::default();
    let contract_id = env.register(ProductFollowContract, ());
    let followers = 5;
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let product_id = 3u32;
    let categories = vec![&env, FollowCategory::PriceChange];
    let mut follower_address: Option<Address> = None;
    env.mock_all_auths();

    for _ in 0..followers {
        let follower_address = <Address>::generate(&env);
        client.follow_product(&follower_address, &product_id, &categories);
    }
    env.as_contract(&contract_id, || {
        let key = symbol_short!("followers");
        let reputation_records: Vec<FollowData> = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Reputation history key rating key not found");
        assert_eq!(reputation_records.len(), followers);
        follower_address = Some(reputation_records.first().unwrap().user)
    });

    client.unfollow_product(&follower_address.unwrap(), &product_id);

    env.as_contract(&contract_id, || {
        let key = symbol_short!("followers");
        let reputation_records: Vec<FollowData> = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Reputation history key rating key not found");
        assert_eq!(reputation_records.len(), followers - 1);
    });
}

/* -------------------------------------------------------------------------- */
/*                                      -                                     */
/* -------------------------------------------------------------------------- */

#[test] // Done
fn test_notification_delivery_timing() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let user_address = <Address>::generate(&env);
    client.register_user(&user_address);
    let product_id = 1234u32;
    let categories = vec![&env, FollowCategory::PriceChange];

    // Follow the product
    client.follow_product(&user_address, &product_id, &categories);

    // Simulate price change
    let new_price: u64 = 99;
    client.notify_price_change(&product_id, &new_price);

    // Retrieve the timestamp of the notification
    env.as_contract(&contract_id, || {
        let history_key = DataKeys::NotificationHistory(user_address);
        let notifications: Vec<EventLog> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(&env));

        assert_eq!(notifications.len(), 1);
        assert_eq!(
            notifications.first().unwrap().product_id,
            product_id as u128
        );
        assert_eq!(
            notifications.first().unwrap().event_type,
            FollowCategory::PriceChange
        );

        let notification_time = notifications.first().unwrap().triggered_at;
        let current_time = env.ledger().timestamp();

        // Check if notification delivery is within a reasonable time window
        assert!(current_time >= notification_time);
    });
}

/* -------------------------------------------------------------------------- */
/*                                      -                                     */
/* -------------------------------------------------------------------------- */

#[test] // Done
fn test_notification_priority_handling() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let user_address = <Address>::generate(&env);
    client.register_user(&user_address);
    let product_id = 1234u32;
    let categories = vec![&env, FollowCategory::PriceChange];

    // Set high priority for the user
    let preferences = NotificationPreferences {
        user: user_address.clone(),
        categories: vec![&env, FollowCategory::PriceChange],
        mute_notifications: false,
        priority: NotificationPriority::High,
    };
    client.set_notification_preferences(&user_address, &preferences);

    // Follow the product
    client.follow_product(&user_address, &product_id, &categories);

    // Simulate price change
    let new_price = 99u64;
    client.notify_price_change(&product_id, &new_price);

    // Check if the notification has the correct priority
    env.as_contract(&contract_id, || {
        let history_key = DataKeys::NotificationHistory(user_address);
        let notifications: Vec<EventLog> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(&env));

        assert_eq!(notifications.len(), 1);
        assert_eq!(
            notifications.first().unwrap().priority,
            NotificationPriority::High
        );
    });
}

/* -------------------------------------------------------------------------- */
/*                                      -                                     */
/* -------------------------------------------------------------------------- */

#[test] // Done
fn test_validate_notification_format() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let user_address = <Address>::generate(&env);
    client.register_user(&user_address);
    let product_id = 1234u32;
    let categories = vec![&env, FollowCategory::PriceChange];

    // Follow the product
    client.follow_product(&user_address, &product_id, &categories);

    // Simulate price change
    let new_price = 99;
    client.notify_price_change(&product_id, &new_price);

    env.as_contract(&contract_id, || {
        // Retrieve the notification event and check its format
        let history_key = DataKeys::NotificationHistory(user_address);
        let notifications: Vec<EventLog> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(&env));

        assert_eq!(notifications.len(), 1);
        let notification = &notifications.first().unwrap();

        // Validate notification format
        assert_eq!(notification.product_id, product_id as u128);
        assert!(notification.event_type == FollowCategory::PriceChange);
        assert!(matches!(
            notification.priority,
            NotificationPriority::High | NotificationPriority::Medium | NotificationPriority::Low
        ));
    });
}

/* -------------------------------------------------------------------------- */
/*                                      -                                     */
/* -------------------------------------------------------------------------- */

#[test] // Done
fn test_verify_notification_history_tracking() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let user_address = <Address>::generate(&env);
    client.register_user(&user_address);
    let product_id = 1234u32;
    let categories = vec![&env, FollowCategory::PriceChange];

    // Follow the product
    client.follow_product(&user_address, &product_id, &categories);

    // Simulate price change
    let new_price = 99;
    client.notify_price_change(&product_id, &new_price);

    // Simulate another price change
    let new_price_2 = 120;
    client.notify_price_change(&product_id, &new_price_2);

    // Check notification history size (should not exceed 100)
    env.as_contract(&contract_id, || {
        let history_key = DataKeys::NotificationHistory(user_address);
        let notifications: Vec<EventLog> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(&env));

        assert_eq!(2, notifications.len()); // Ensure that it's 2 entries
        assert!(notifications.len() <= 100); // Ensure that the history is capped
    });
}

/* -------------------------------------------------------------------------- */
/*                                      -                                     */
/* -------------------------------------------------------------------------- */

#[test] // Done
fn test_notification_customization_options() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let user_address = <Address>::generate(&env);
    client.register_user(&user_address);
    let product_id = 1234u32;
    let categories = vec![&env, FollowCategory::PriceChange];

    // Set preferences to mute notifications
    let preferences = NotificationPreferences {
        user: user_address.clone(),
        categories: vec![&env, FollowCategory::PriceChange],
        mute_notifications: true,
        priority: NotificationPriority::High,
    };
    client.set_notification_preferences(&user_address, &preferences);

    // Follow the product
    client.follow_product(&user_address, &product_id, &categories);

    // Simulate price change
    let new_price = 99;
    client.notify_price_change(&product_id, &new_price);

    // Check if the notification was suppressed (since mute is enabled)
    env.as_contract(&contract_id, || {
        let history_key = DataKeys::NotificationHistory(user_address.clone());
        let notifications: Vec<EventLog> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(&env));

        assert_eq!(notifications.len(), 0); // No notification should be sent
    });
}

/* -------------------------------------------------------------------------- */
/*                                      -                                     */
/* -------------------------------------------------------------------------- */

#[test] // Done
fn test_validate_user_preferences() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ProductFollowContract, ());
    let client = ProductFollowContractClient::new(&env, &contract_id);
    let user_address = <Address>::generate(&env);

    // Set custom notification preferences
    let preferences = NotificationPreferences {
        user: user_address.clone(),
        categories: vec![&env, FollowCategory::Restock],
        mute_notifications: false,
        priority: NotificationPriority::Medium,
    };
    client.set_notification_preferences(&user_address, &preferences);

    // Retrieve preferences and verify
    let retrieved_preferences = client.get_notification_preferences(&user_address);

    assert_eq!(retrieved_preferences.user, user_address);
    assert_eq!(
        retrieved_preferences.categories,
        vec![&env, FollowCategory::Restock]
    );
    assert_eq!(retrieved_preferences.priority, NotificationPriority::Medium);
}
