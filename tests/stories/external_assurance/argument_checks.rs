use cucumber::{given, then};
use specforge::argument::AssuranceArgument;
use specforge::ledger::{EvidenceLink, EvidenceRelation};
use specforge::verify::FormalOutcome;

use super::argument::{append_formal_evidence, built_argument};
use crate::Forge;

#[given(expr = "formal proved evidence exists for {string} and claim {string} with a supports link")]
fn given_formal_proved_with_supports_link(world: &mut Forge, property_id: String, claim_id: String) {
    world.external_assurance.evidence_counter += 1;
    let id: String = format!("EV-PROVED-{}", world.external_assurance.evidence_counter);
    append_formal_evidence(world, &id, &property_id, &claim_id, FormalOutcome::Proved, None);
    let link: EvidenceLink =
        EvidenceLink::seal(id, claim_id, EvidenceRelation::Supports, "formal proof supports claim".to_string()).expect("supports link seals");
    world.evidence_ledger.append_link(link).expect("supports link appends");
}

#[then(expr = "the assurance argument has {int} defeaters")]
fn argument_defeater_count(world: &mut Forge, expected: usize) {
    let argument: &AssuranceArgument = built_argument(world);
    assert_eq!(argument.defeaters.len(), expected, "unexpected defeaters: {:?}", argument.defeaters);
}
