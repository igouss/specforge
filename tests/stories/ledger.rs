use std::cell::Cell;

use cucumber::{given, then, when};

tokio::task_local! {
    pub static RAN: Cell<usize>;
}

#[derive(Debug, Default, cucumber::World)]
pub struct Ledger {
    accounts: Vec<i64>,
}

#[given("an account with 100 credits")]
fn account(world: &mut Ledger) {
    RAN.with(|ran: &Cell<usize>| ran.set(ran.get() + 1));
    world.accounts = vec![100, 0];
}

#[when("60 credits are transferred")]
fn transferred(world: &mut Ledger) {
    world.accounts = world
        .accounts
        .iter()
        .zip([-60, 60])
        .map(|(credits, moved): (&i64, i64)| credits + moved)
        .collect();
}

#[then(expr = "the total is {int}")]
fn total_is(world: &mut Ledger, total: i64) {
    assert_eq!(world.accounts.iter().sum::<i64>(), total);
}
