//! Tests for the per-ship power-screen value derivation: Clipper-shaped
//! fixtures assert pool parsing, consumption summing, in-use rounding,
//! display ordering, and the emitted binding paths match the
//! reference-verified at-rest model (4/6/4 pips, 2/3/2 in use, 7 pools).

use super::*;

fn clipper_pool_icons() -> HashMap<String, String> {
    [
        ("WeaponGun", "UI/Textures/Vector/General/CommonIcons/icon_common_weapon_gun.svg"),
        ("FlightController", "UI/Textures/Vector/Ships/ProfessionScreens/Engineering/Engineering_Icon_ItemThrusters.svg"),
        ("Shield", "UI/Textures/Vector/General/CommonIcons/icon_common_generator_shield.svg"),
        ("TractorBeam", "UI/Textures/Vector/General/CommonIcons/icon_common_tractor beam.svg"),
        ("TowingBeam", "UI/Textures/Vector/General/CommonIcons/icon_common_tractor beam.svg"),
        ("WeaponMining", "UI/Textures/Vector/General/CommonIcons/icon_common_mining.svg"),
        ("SalvageHead", "UI/Textures/Vector/General/CommonIcons/icon_common_salvage.svg"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}


fn clipper_pools_json() -> Json {
    serde_json::json!({
        "_RecordValue_": {
            "Components": [
                {
                    "_Type_": "SItemPortContainerComponentParams",
                    "resourceNetworkPowerPools": {
                        "itemPools": [
                            {"_Type_": "FixedPowerPool", "itemType": "WeaponGun", "poolSize": 4},
                            {"_Type_": "DynamicPowerPool", "itemType": "Shield", "maxItemCount": 2},
                            {"_Type_": "DynamicPowerPool", "itemType": "FlightController", "maxItemCount": -1},
                            {"_Type_": "DynamicPowerPool", "itemType": "TractorBeam", "maxItemCount": -1},
                            {"_Type_": "DynamicPowerPool", "itemType": "TowingBeam", "maxItemCount": -1},
                            {"_Type_": "DynamicPowerPool", "itemType": "WeaponMining", "maxItemCount": -1},
                            {"_Type_": "DynamicPowerPool", "itemType": "SalvageHead", "maxItemCount": -1},
                            null
                        ]
                    }
                }
            ]
        }
    })
}

fn item_json(attach_type: &str, power_units: u64, overheat: Option<f64>) -> Json {
    let mut components = vec![
        serde_json::json!({
            "_Type_": "SAttachableComponentParams",
            "AttachDef": {"Type": attach_type}
        }),
        serde_json::json!({
            "_Type_": "ItemResourceComponentParams",
            "states": [
                {
                    "name": "Online",
                    "deltas": [
                        {
                            "consumption": {
                                "resource": "Power",
                                "resourceAmountPerSecond": {
                                    "_Type_": "SPowerSegmentResourceUnit",
                                    "units": power_units
                                }
                            }
                        },
                        {
                            // Coolant consumption must not count as power.
                            "consumption": {
                                "resource": "Coolant",
                                "resourceAmountPerSecond": {
                                    "_Type_": "SStandardResourceUnit",
                                    "standardResourceUnits": 5.0
                                }
                            }
                        }
                    ]
                }
            ]
        }),
    ];
    if let Some(overheat) = overheat {
        components.push(serde_json::json!({
            "_Type_": "SEntityPhysicsControllerParams",
            "PhysType": {
                "temperature": {
                    "itemResourceParams": {"overheatTemperature": overheat}
                }
            }
        }));
    }
    serde_json::json!({"_RecordValue_": {"Components": components}})
}

fn clipper_fitted_items() -> Vec<Json> {
    vec![
        item_json("Shield", 2, Some(372.0)),
        item_json("Shield", 2, Some(372.0)),
        item_json("FlightController", 6, None),
        // A power plant GENERATES power; its record carries no power
        // consumption delta and must not contribute pips.
        item_json("PowerPlant", 0, Some(372.0)),
    ]
}

#[test]
fn vehicle_pools_parse_with_fixed_sizes() {
    let pools = power_pools_from_vehicle(&clipper_pools_json());
    assert_eq!(pools.len(), 7, "null pool entries are skipped");
    assert_eq!(pools[0], PowerPool { item_type: "WeaponGun".into(), fixed_size: Some(4) });
    assert_eq!(pools[1], PowerPool { item_type: "Shield".into(), fixed_size: None });
}

#[test]
fn item_power_units_sum_only_power_segment_units() {
    let item = item_json("Shield", 2, None);
    assert_eq!(item_power_units(&item), 2);
    assert_eq!(item_attach_type(&item).as_deref(), Some("Shield"));
}

#[test]
fn derived_clipper_paths_match_reference_model() {
    let pools = power_pools_from_vehicle(&clipper_pools_json());
    let items = clipper_fitted_items();
    let paths = derive_power_paths(&pools, &items, &PoolDefaults::default(), &clipper_pool_icons());

    assert_eq!(paths.get("piplist"), Some(&UiValue::Int(7)));
    assert_eq!(
        paths.get("resourcenetworkui/powermanagement/pipsLengthMax"),
        Some(&UiValue::Int(6)),
        "largest stack = engines"
    );

    // Display order: weapons, engines (FlightController), shields, then rest.
    assert_eq!(
        paths.get("piplist/[0000]/itemicon"),
        Some(&UiValue::Str(
            "UI/Textures/Vector/General/CommonIcons/icon_common_weapon_gun.svg".into()
        ))
    );
    assert_eq!(
        paths.get("piplist/[0001]/itemicon"),
        Some(&UiValue::Str(
            "UI/Textures/Vector/Ships/ProfessionScreens/Engineering/Engineering_Icon_ItemThrusters.svg".into()
        ))
    );
    assert_eq!(
        paths.get("piplist/[0002]/itemicon"),
        Some(&UiValue::Str(
            "UI/Textures/Vector/General/CommonIcons/icon_common_generator_shield.svg".into()
        ))
    );

    // 4/6/4 totals, 2/3/2 in use (bright bottom-up), selected = highest lit.
    for (i, total, in_use) in [(0u32, 4i64, 2i64), (1, 6, 3), (2, 4, 2)] {
        assert_eq!(paths.get(&format!("piplist/[{i:04}]/piplist")), Some(&UiValue::Int(total)));
        assert_eq!(
            paths.get(&format!("piplist/[{i:04}]/piplist/selectedindex")),
            Some(&UiValue::Int(in_use - 1))
        );
        for j in 0..total {
            let expected = i64::from(j < in_use);
            assert_eq!(
                paths.get(&format!("piplist/[{i:04}]/piplist/[{j:04}]/pipstate")),
                Some(&UiValue::Int(expected)),
                "pool {i} pip {j}"
            );
        }
    }

    // Heat gauges: engines + shields (dynamic, with consumers, overheat from
    // the item where modelled); weapons (fixed) and empty pools hide theirs.
    assert_eq!(
        paths.get("piplist/[0001]/tempindicator/overheattemp"),
        Some(&UiValue::Float(372.0)),
        "engines fall back to the small-component overheat family"
    );
    assert_eq!(
        paths.get("piplist/[0002]/tempindicator/overheattemp"),
        Some(&UiValue::Float(372.0)),
        "shield overheat comes from the fitted item"
    );
    assert_eq!(
        paths.get("piplist/[0000]/tempindicator/maxtemp"),
        Some(&UiValue::Float(0.0)),
        "fixed weapons pool hides the gauge"
    );
    assert_eq!(
        paths.get("piplist/[0003]/tempindicator/maxtemp"),
        Some(&UiValue::Float(0.0)),
        "empty pools hide the gauge"
    );
    // Empty pools render an icon and no pips.
    assert_eq!(paths.get("piplist/[0003]/piplist"), Some(&UiValue::Int(0)));
    assert!(paths.get("piplist/[0003]/piplist/selectedindex").is_none());
}

/// The battery card's `batteryremaining` / `batterytotal` engine variables
/// come from the fitted Battery items: none fitted (the Clipper) shows
/// "0 / 0" in the in-game reference; fitted batteries count as charged at
/// rest.
#[test]
fn battery_counts_come_from_fitted_items() {
    let pools = power_pools_from_vehicle(&clipper_pools_json());

    let no_batteries = clipper_fitted_items();
    let paths =
        derive_power_paths(&pools, &no_batteries, &PoolDefaults::default(), &clipper_pool_icons());
    assert_eq!(paths.get("batteryremaining"), Some(&UiValue::Int(0)));
    assert_eq!(paths.get("batterytotal"), Some(&UiValue::Int(0)));

    let mut with_batteries = clipper_fitted_items();
    with_batteries.push(item_json("Battery", 0, None));
    with_batteries.push(item_json("Battery", 0, None));
    let paths = derive_power_paths(
        &pools,
        &with_batteries,
        &PoolDefaults::default(),
        &clipper_pool_icons(),
    );
    assert_eq!(paths.get("batteryremaining"), Some(&UiValue::Int(2)));
    assert_eq!(paths.get("batterytotal"), Some(&UiValue::Int(2)));
}

/// The gauge scale ceiling tracks the ITEM's real overheat threshold (the
/// Clipper's flight controller overheats at 450 K, its shields at 372 K), so
/// the heat-bar band ratios stay coherent (overheat never exceeds the scale).
#[test]
fn gauge_scale_tracks_item_overheat_threshold() {
    let pools = power_pools_from_vehicle(&clipper_pools_json());
    let items = vec![
        item_json("Shield", 2, Some(372.0)),
        item_json("FlightController", 6, Some(450.0)),
    ];
    let paths = derive_power_paths(&pools, &items, &PoolDefaults::default(), &clipper_pool_icons());

    assert_eq!(
        paths.get("piplist/[0001]/tempindicator/overheattemp"),
        Some(&UiValue::Float(450.0))
    );
    assert_eq!(
        paths.get("piplist/[0001]/tempindicator/maxtemp"),
        Some(&UiValue::Float(518.0)),
        "ceiling = overheat + reference-calibrated headroom"
    );
    assert_eq!(
        paths.get("piplist/[0002]/tempindicator/maxtemp"),
        Some(&UiValue::Float(440.0)),
        "shield scale matches the reference-calibrated 372+68"
    );
}
