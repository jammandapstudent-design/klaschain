#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::{StellarAssetClient, TokenClient},
    symbol_short, Address, Env,
};

fn setup_env() -> (Env, Address, Address, Address, TokenClient<'static>, StellarAssetClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(KlasChainContract, ());

    let admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_client = TokenClient::new(&env, &token_address);
    let token_admin_client = StellarAssetClient::new(&env, &token_address);

    let client = KlasChainContractClient::new(&env, &contract_id);
    client.initialize(&admin, &token_address);

    (env, contract_id, admin, token_address, token_client, token_admin_client)
}

fn advance_ledger(env: &Env, seconds: u64) {
    let current_time = env.ledger().timestamp();
    env.ledger().set(LedgerInfo {
        timestamp: current_time + seconds,
        protocol_version: 22,
        sequence_number: env.ledger().sequence() + 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3_110_400,
    });
}

// ──────────────────────────────────────────────
// Test 1 — Happy Path
// Create circle, join, contribute, redistribute
// ──────────────────────────────────────────────
#[test]
fn test_happy_path_ub_circle() {
    let (env, contract_id, _admin, _token_address, token_client, token_admin) = setup_env();
    let client = KlasChainContractClient::new(&env, &contract_id);
    
    let creator = Address::generate(&env);
    let student2 = Address::generate(&env);
    let student3 = Address::generate(&env);

    // Initial funding for students
    token_admin.mint(&creator, &100_i128);
    token_admin.mint(&student2, &100_i128);
    token_admin.mint(&student3, &100_i128);

    // Creator makes a circle
    let name = symbol_short!("TEST");
    let circle_id = client.create_circle(&creator, &name, &10_i128, &3_u32);

    // Others join
    client.join_circle(&student2, &circle_id);
    client.join_circle(&student3, &circle_id);

    // All contribute
    client.contribute(&creator, &circle_id);
    client.contribute(&student2, &circle_id);
    client.contribute(&student3, &circle_id);

    assert_eq!(token_client.balance(&creator), 90_i128);
    
    // Advance time by 7 days
    advance_ledger(&env, 7 * 24 * 60 * 60 + 1);

    // Redistribute
    client.redistribute(&circle_id);

    // 30 USDC pool / 3 members = 10 USDC each. Balances should be back to 100.
    assert_eq!(token_client.balance(&creator), 100_i128);
    assert_eq!(token_client.balance(&student2), 100_i128);
    assert_eq!(token_client.balance(&student3), 100_i128);
}

// ──────────────────────────────────────────────
// Test 2 — Edge Case: Double Contribution
// Member cannot contribute twice in same week
// ──────────────────────────────────────────────
#[test]
#[should_panic(expected = "already contributed this week")]
fn test_double_contribution_fails() {
    let (env, contract_id, _admin, _token_address, _token_client, token_admin) = setup_env();
    let client = KlasChainContractClient::new(&env, &contract_id);
    
    let creator = Address::generate(&env);
    token_admin.mint(&creator, &100_i128);

    let name = symbol_short!("TEST");
    let circle_id = client.create_circle(&creator, &name, &10_i128, &5_u32);

    client.contribute(&creator, &circle_id);
    // Second time should panic
    client.contribute(&creator, &circle_id);
}

// ──────────────────────────────────────────────
// Test 3 — State Verification: Circle State
// Ensure state is updated correctly after contribution
// ──────────────────────────────────────────────
#[test]
fn test_circle_state_updates() {
    let (env, contract_id, _admin, _token_address, _token_client, token_admin) = setup_env();
    let client = KlasChainContractClient::new(&env, &contract_id);
    
    let creator = Address::generate(&env);
    token_admin.mint(&creator, &100_i128);

    let name = symbol_short!("TEST");
    let circle_id = client.create_circle(&creator, &name, &25_i128, &10_u32);

    client.contribute(&creator, &circle_id);

    let circle = client.get_circle(&circle_id);
    assert_eq!(circle.current_pool, 25_i128);
    assert_eq!(circle.contributors_this_week.len(), 1);
    assert_eq!(circle.contributors_this_week.get(0).unwrap(), creator);
}

// ──────────────────────────────────────────────
// Test 4 — Edge Case: Redistribute Too Early
// Must wait 7 days
// ──────────────────────────────────────────────
#[test]
#[should_panic(expected = "redistribution period not yet elapsed")]
fn test_redistribute_too_early_fails() {
    let (env, contract_id, _admin, _token_address, _token_client, token_admin) = setup_env();
    let client = KlasChainContractClient::new(&env, &contract_id);
    
    let creator = Address::generate(&env);
    token_admin.mint(&creator, &100_i128);

    let name = symbol_short!("TEST");
    let circle_id = client.create_circle(&creator, &name, &10_i128, &3_u32);
    client.contribute(&creator, &circle_id);

    // Advance only 3 days
    advance_ledger(&env, 3 * 24 * 60 * 60);

    // Should panic
    client.redistribute(&circle_id);
}

// ──────────────────────────────────────────────
// Test 5 — Edge Case: Join Full Circle
// Cannot join if max_members reached
// ──────────────────────────────────────────────
#[test]
#[should_panic(expected = "circle is full")]
fn test_join_full_circle_fails() {
    let (env, contract_id, _admin, _token_address, _token_client, _token_admin) = setup_env();
    let client = KlasChainContractClient::new(&env, &contract_id);
    
    let creator = Address::generate(&env);
    let name = symbol_short!("TEST");
    // Max members = 1
    let circle_id = client.create_circle(&creator, &name, &10_i128, &1_u32);

    let student2 = Address::generate(&env);
    
    // Should panic because circle max_members is 1 and creator is already in
    client.join_circle(&student2, &circle_id);
}
