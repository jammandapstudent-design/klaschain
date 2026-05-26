#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, Map, Symbol, Vec
};

// ──────────────────────────────────────────────
// Storage Keys
// ──────────────────────────────────────────────

const ADMIN: Symbol = symbol_short!("ADMIN");
const TOKEN: Symbol = symbol_short!("TOKEN");
const CIRCLES: Symbol = symbol_short!("CIRCLES");
const CIRCLE_CTR: Symbol = symbol_short!("CIR_CTR");

// ──────────────────────────────────────────────
// Data Types
// ──────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Circle {
    pub id: u32,
    pub name: Symbol,
    pub contribution_amount: i128,
    pub max_members: u32,
    pub members: Vec<Address>,
    pub current_pool: i128,
    pub contributors_this_week: Vec<Address>,
    pub last_redistribution: u64,
}

// ──────────────────────────────────────────────
// Contract
// ──────────────────────────────────────────────

#[contract]
pub struct KlasChainContract;

#[contractimpl]
impl KlasChainContract {
    // ────────────────────────────────────────
    // initialize
    // ────────────────────────────────────────
    pub fn initialize(env: Env, admin: Address, token_address: Address) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }
        admin.require_auth();

        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&TOKEN, &token_address);
        env.storage().instance().set(&CIRCLE_CTR, &0_u32);
        
        let circles: Map<u32, Circle> = Map::new(&env);
        env.storage().instance().set(&CIRCLES, &circles);
    }

    // ────────────────────────────────────────
    // create_circle
    // ────────────────────────────────────────
    pub fn create_circle(env: Env, creator: Address, name: Symbol, contribution_amount: i128, max_members: u32) -> u32 {
        creator.require_auth();

        if contribution_amount <= 0 {
            panic!("contribution amount must be positive");
        }
        if max_members == 0 {
            panic!("max_members must be > 0");
        }

        let mut ctr: u32 = env.storage().instance().get(&CIRCLE_CTR).unwrap();
        ctr += 1;

        let mut circles: Map<u32, Circle> = env.storage().instance().get(&CIRCLES).unwrap();
        
        let mut members = Vec::new(&env);
        members.push_back(creator.clone());

        let circle = Circle {
            id: ctr,
            name,
            contribution_amount,
            max_members,
            members,
            current_pool: 0,
            contributors_this_week: Vec::new(&env),
            last_redistribution: env.ledger().timestamp(),
        };

        circles.set(ctr, circle);
        env.storage().instance().set(&CIRCLES, &circles);
        env.storage().instance().set(&CIRCLE_CTR, &ctr);

        ctr
    }

    // ────────────────────────────────────────
    // join_circle
    // ────────────────────────────────────────
    pub fn join_circle(env: Env, student: Address, circle_id: u32) {
        student.require_auth();

        let mut circles: Map<u32, Circle> = env.storage().instance().get(&CIRCLES).unwrap();
        let mut circle = circles.get(circle_id).expect("circle not found");

        if circle.members.len() >= circle.max_members {
            panic!("circle is full");
        }
        if circle.members.contains(&student) {
            panic!("already a member");
        }

        circle.members.push_back(student);
        circles.set(circle_id, circle);
        env.storage().instance().set(&CIRCLES, &circles);
    }

    // ────────────────────────────────────────
    // contribute
    // MVP Core Feature
    // ────────────────────────────────────────
    pub fn contribute(env: Env, student: Address, circle_id: u32) {
        student.require_auth();

        let mut circles: Map<u32, Circle> = env.storage().instance().get(&CIRCLES).unwrap();
        let mut circle = circles.get(circle_id).expect("circle not found");

        if !circle.members.contains(&student) {
            panic!("not a member of this circle");
        }
        if circle.contributors_this_week.contains(&student) {
            panic!("already contributed this week");
        }

        let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
        let contract_address = env.current_contract_address();

        // Transfer USDC from student → contract
        token::Client::new(&env, &token_address).transfer(
            &student,
            &contract_address,
            &circle.contribution_amount,
        );

        circle.current_pool += circle.contribution_amount;
        circle.contributors_this_week.push_back(student);

        circles.set(circle_id, circle);
        env.storage().instance().set(&CIRCLES, &circles);
    }

    // ────────────────────────────────────────
    // redistribute
    // MVP Core Feature
    // ────────────────────────────────────────
    pub fn redistribute(env: Env, circle_id: u32) {
        let mut circles: Map<u32, Circle> = env.storage().instance().get(&CIRCLES).unwrap();
        let mut circle = circles.get(circle_id).expect("circle not found");

        let now = env.ledger().timestamp();
        let seven_days: u64 = 7 * 24 * 60 * 60;

        // Ensure 7 days have passed, unless this is a test where we allow immediate redistribution if pool > 0 for demo purposes.
        // For production, we'd enforce the 7 days strictly. 
        // We will enforce it strictly here, but tests will need to advance ledger time.
        if now < circle.last_redistribution + seven_days {
             panic!("redistribution period not yet elapsed");
        }

        if circle.contributors_this_week.len() == 0 {
             // Reset for next week even if no contributions
             circle.last_redistribution = now;
             circles.set(circle_id, circle);
             env.storage().instance().set(&CIRCLES, &circles);
             return;
        }

        let num_members_total = circle.members.len() as i128;
        let amount_per_member = circle.current_pool / num_members_total;
        
        // In a real UBI circle, the pool might be distributed evenly among ALL active members,
        // or just the contributors. The spec says "redistributes the total pool equally to all active members".
        // Let's assume 'members' list are the active members.

        let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &token_address);

        if amount_per_member > 0 {
             for member in circle.members.iter() {
                 token_client.transfer(
                     &contract_address,
                     &member,
                     &amount_per_member,
                 );
             }
        }

        // Handle dust (remainder). In this simple MVP, we just leave it in the pool.
        let distributed_total = amount_per_member * num_members_total;
        circle.current_pool -= distributed_total; 
        
        // Reset for next week
        circle.contributors_this_week = Vec::new(&env);
        circle.last_redistribution = now;

        circles.set(circle_id, circle);
        env.storage().instance().set(&CIRCLES, &circles);
    }
    
    pub fn get_circle(env: Env, circle_id: u32) -> Circle {
        let circles: Map<u32, Circle> = env.storage().instance().get(&CIRCLES).unwrap();
        circles.get(circle_id).expect("circle not found")
    }
}

#[cfg(test)]
mod test;
