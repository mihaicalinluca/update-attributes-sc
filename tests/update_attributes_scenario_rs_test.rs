use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();

    blockchain.register_contract("mxsc:output/update-attributes.mxsc.json", update_attributes::ContractBuilder);
    blockchain
}

#[test]
fn empty_rs() {
    world().run("scenarios/update_attributes.scen.json");
}
